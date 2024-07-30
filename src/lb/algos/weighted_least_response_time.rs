use crate::lb::Loadbalancer;
use crate::Server;

pub struct WeightedLeastResponseTime {}

impl WeightedLeastResponseTime {
    pub fn new() -> Self {
        WeightedLeastResponseTime {}
    }
}

impl Loadbalancer for WeightedLeastResponseTime {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        let min_index = servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32 / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}
