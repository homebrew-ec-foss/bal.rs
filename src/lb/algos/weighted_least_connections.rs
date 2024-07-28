use crate::lb::LoadBalancer;

pub struct WeightedLeastConnections {

}

impl WeightedLeastConnections {
    pub fn new() -> Self {
        WeightedLeastConnections {}
    }
}

impl LoadBalancer for WeightedLeastConnections {
    fn get_index(&mut self, config: std::sync::Arc<&std::sync::MutexGuard<crate::Config>>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections / server.weight);
        match min_index {
            Some((index, _)) => Some(index),
            None => None
        }
    }
}