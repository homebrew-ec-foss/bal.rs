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
        let min_index = servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections)
            .map(|(index, _)| index);
        min_index
    }
}
