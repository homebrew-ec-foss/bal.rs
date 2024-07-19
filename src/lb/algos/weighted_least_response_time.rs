use hyper::{client, Body, Client, Request, Response, Uri};
use hyper::client::HttpConnector;
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, sleep, Duration};
use crate::lb::LoadBalancer;

pub struct weighted_least_response_time {
    servers: Arc<Mutex<Vec<server>>>,
}

impl weighted_least_response_time {
    pub fn new(backends: Vec<hyper::Uri>, weights: Vec<u32>) -> Self {
        weighted_least_response_time {
            servers: Arc::new(Mutex::new(backends
                                .into_iter()
                                .zip(weights.iter())
                                .map(|(backend, weight)| server::new(backend, weight))
                                .collect())),
        }
    }
    async fn update_response_time(&mut self) {
        println!("In");
        for server in self.servers.lock().unwrap().iter_mut() {
            println!("before: {} {}  ...", server.uri, server.response_time.as_millis());
            server.update_response_time();
            println!("after: {} {}  ...", server.uri, server.response_time.as_millis());
        }
    }
}

impl LoadBalancer for weighted_least_response_time {
    fn get_server(&mut self) -> Uri {
        // self.update_response_time().await;
        self.servers.lock().unwrap().iter()
            .map(|server| {
                let weighted_time = (server.response_time.as_millis() as f64) / (server.weight as f64);
                println!("{} {} {}", server.uri, weighted_time, server.response_time.as_millis());
                (server.uri.clone(), weighted_time)
            })
            .min_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(uri, _)| uri)
            .unwrap_or_else(|| self.servers.lock().unwrap().get(0).unwrap().uri.clone())
    }
}

struct server {
    uri: hyper::Uri,
    weight: u32,
    response_time: Duration,
}

impl server {
    fn new(uri: hyper::Uri, weight: &u32) -> Self {
        server {
            uri: uri,
            weight: *weight,
            response_time: Duration::from_secs(0),
        }
    }
    async fn update_response_time(&mut self) -> &Self {
        let client = Client::new();
        let start = std::time::Instant::now();

        self.response_time = match timeout(Duration::from_secs(2), client.get(self.uri.clone())).await {
            Ok(Ok(_)) => start.elapsed(),
            _ => Duration::from_secs(u64::MAX),
        };

        println!("{}", self.response_time.as_millis());

        self
    }
}