use hyper::{Server, Uri};
use crate::lb::LoadBalancer;

pub struct WeightedLeastConnections {
    servers: Vec<server>,
}

pub struct server {
    uri: hyper::Uri,
    pub connections: u32,
    max_connections: u32,
    weight: u32,
}

impl WeightedLeastConnections {
    pub fn new(backends: Vec<Uri>, weights: Vec<u32>, max_connections: Vec<u32>) -> Self {
        WeightedLeastConnections {
            servers: backends.iter()
                .enumerate()
                .map(|(index, uri)| server {
                    uri: uri.clone(),
                    connections: 0,
                    max_connections: max_connections.get(index).unwrap().clone(),
                    weight: weights.get(index).unwrap().clone(),

                })
                .collect()
        }
    }
}

impl LoadBalancer for WeightedLeastConnections {
    fn get_server(&mut self) -> Uri {
        let min_index = self.servers.iter()
            .enumerate()
            .map(|(index, server)| {
                let weighted_score = server.connections / server.weight;
                (index, weighted_score)
            })
            .min_by_key(|(_, score)| *score)
            .unwrap_or((0, u32::MAX)); // Default to index 0 if no servers are available

        // Increment the connection count of the selected server
        let selected_server = &mut self.servers[min_index.0];
        selected_server.connections += 1;

        selected_server.uri.clone()
    }
}
