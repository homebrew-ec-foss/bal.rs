use hyper::Uri;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
// use tokio::sync::;
use crate::lb::LoadBalancer;

pub struct weighted_least_connections {
    servers: Arc<Mutex<Vec<Server>>>,
}

impl weighted_least_connections {
    pub fn new(backends: Vec<Uri>, weights: Vec<u32>) -> Self {
        weighted_least_connections {
            servers: Arc::new(Mutex::new(
                backends.into_iter()
                    .zip(weights.into_iter())
                    .map(|(backend, weight)| Server::new(backend, weight))
                    .collect::<Vec<Server>>()
            )),
        }
    }

    pub fn add_backend(&self, backend: Uri, weight: u32) {
        let mut servers = self.servers.lock().unwrap();
        servers.push(Server::new(backend, weight));
    }

    pub fn remove_backend(&self, backend: &Uri) {
        let mut servers = self.servers.lock().unwrap();
        servers.retain(|server| &server.uri != backend);
    }
}

impl LoadBalancer for weighted_least_connections {
    fn get_server(&mut self) -> Uri {
        let servers = self.servers.lock().unwrap();
        servers.iter()
            .map(|server| {
                let weighted_connections = server.active_connections.load(Ordering::SeqCst) as f64 / server.weight as f64;
                (server.uri.clone(), weighted_connections)
            })
            .min_by(|(_, c1), (_, c2)| c1.partial_cmp(c2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(uri, _)| uri)
            .unwrap_or_else(|| servers.first().unwrap().uri.clone())
    }
}

struct Server {
    uri: Uri,
    weight: u32,
    active_connections: Arc<AtomicUsize>,
}

impl Server {
    fn new(uri: Uri, weight: u32) -> Self {
        Server {
            uri,
            weight,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }
}
