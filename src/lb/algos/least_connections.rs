use crate::lb::LoadBalancer;
use crate::Config;
use std::sync::MutexGuard;

pub struct LeastConnections {}

impl LeastConnections {
    pub fn new() -> Self {
        LeastConnections {}
    }
}

impl LoadBalancer for LeastConnections {
    fn get_index(&mut self, config: &MutexGuard<Config>) -> Option<usize> {
        let min_index = config
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections)
            .map(|(index, _)| index);
        min_index
    }
}
