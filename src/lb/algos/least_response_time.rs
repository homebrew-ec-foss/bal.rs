use crate::lb::Loadbalancer;
use crate::Server;

pub struct LeastResponseTime {}

impl LeastResponseTime {
    pub fn new() -> Self {
        LeastResponseTime {}
    }
}

impl Loadbalancer for LeastResponseTime {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        if servers.len() == 0 {
            return None;
        }
        let min_index = servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.response_time.as_millis() as u32)
            .map(|(index, _)| index);
        min_index
    }
}
