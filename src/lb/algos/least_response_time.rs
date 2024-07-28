use crate::lb::Loadbalancer;
use crate::LoadBalancer;
use std::sync::MutexGuard;

pub struct LeastResponseTime {}

impl LeastResponseTime {
    pub fn new() -> Self {
        LeastResponseTime {}
    }
}

impl Loadbalancer for LeastResponseTime {
    fn get_index(&mut self, lb: &MutexGuard<LoadBalancer>) -> Option<usize> {
        let min_index = lb
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32)
            .map(|(index, _)| index);
        min_index
    }
}
