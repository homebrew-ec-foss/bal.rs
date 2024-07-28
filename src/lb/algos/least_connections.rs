use crate::lb::LoadBalancer;
use std::sync::MutexGuard;
use crate::Config;

pub struct LeastConnections {

}

impl LeastConnections {
    pub fn new() -> Self {
        LeastConnections {}
    }
}

impl LoadBalancer for LeastConnections {
    fn get_index(&mut self, config: &MutexGuard<Config>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections)
            .map(|(index, _)| index);
        min_index
    }
}