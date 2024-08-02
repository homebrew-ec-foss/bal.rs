use crate::lb::Loadbalancer;
use crate::Server;

pub struct LeastConnections {}

impl LeastConnections {
    pub fn new() -> Self {
        LeastConnections {}
    }
}

impl Loadbalancer for LeastConnections {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        if servers.len() == 0 {
            return None;
        }
        let min_index = servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections)
            .map(|(index, _)| index);
        min_index
    }
}
