use crate::lb::LoadBalancer;
use std::sync::MutexGuard;
use crate::Config;

pub struct WeightedLeastConnections {

}

impl WeightedLeastConnections {
    pub fn new() -> Self {
        WeightedLeastConnections {}
    }
}

impl LoadBalancer for WeightedLeastConnections {
    fn get_index(&mut self, config: &MutexGuard<Config>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}