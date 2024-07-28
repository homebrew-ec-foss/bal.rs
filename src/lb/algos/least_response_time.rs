use crate::lb::LoadBalancer;
use crate::Config;
use std::sync::MutexGuard;

pub struct LeastResponseTime {}

impl LeastResponseTime {
    pub fn new() -> Self {
        LeastResponseTime {}
    }
}

impl LoadBalancer for LeastResponseTime {
    fn get_index(&mut self, config: &MutexGuard<Config>) -> Option<usize> {
        let min_index = config
            .servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32)
            .map(|(index, _)| index);
        min_index
    }
}
