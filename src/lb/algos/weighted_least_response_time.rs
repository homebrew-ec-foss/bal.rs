use crate::lb::LoadBalancer;
use std::sync::MutexGuard;

pub struct WeightedLeastResponseTime {

}

impl WeightedLeastResponseTime {
    pub fn new() -> Self {
        WeightedLeastResponseTime {}
    }
}

impl LoadBalancer for WeightedLeastResponseTime {
    fn get_index(&mut self, config: &MutexGuard<crate::Config>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32 / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}