use crate::lb::Loadbalancer;
use crate::LoadBalancer;
use std::sync::MutexGuard;

pub struct WeightedLeastConnections {}

impl WeightedLeastConnections {
    pub fn new() -> Self {
        WeightedLeastConnections {}
    }
}

impl Loadbalancer for WeightedLeastConnections {
    fn get_index(&mut self, lb: &MutexGuard<LoadBalancer>) -> Option<usize> {
        let min_index = lb
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}
