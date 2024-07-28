use crate::lb::Loadbalancer;
use std::sync::MutexGuard;

pub struct WeightedLeastResponseTime {}

impl WeightedLeastResponseTime {
    pub fn new() -> Self {
        WeightedLeastResponseTime {}
    }
}

impl Loadbalancer for WeightedLeastResponseTime {
    fn get_index(&mut self, lb: &MutexGuard<crate::LoadBalancer>) -> Option<usize> {
        let min_index = lb
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32 / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}
