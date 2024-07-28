use crate::lb::Loadbalancer;
use crate::LoadBalancer;
use std::sync::MutexGuard;

pub struct LeastConnections {}

impl LeastConnections {
    pub fn new() -> Self {
        LeastConnections {}
    }
}

impl Loadbalancer for LeastConnections {
    fn get_index(&mut self, lb: &MutexGuard<LoadBalancer>) -> Option<usize> {
        let min_index = lb
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections)
            .map(|(index, _)| index);
        min_index
    }
}
