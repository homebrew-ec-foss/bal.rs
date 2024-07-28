use crate::lb::LoadBalancer;

pub struct WeightedLeastResponseTime {

}

impl WeightedLeastResponseTime {
    pub fn new() -> Self {
        WeightedLeastResponseTime {}
    }
}

impl LoadBalancer for WeightedLeastResponseTime {
    fn get_index(&mut self, config: std::sync::Arc<&std::sync::MutexGuard<crate::Config>>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32 / server.weight);
        match min_index {
            Some((index, _)) => Some(index),
            None => None
        }
    }
}