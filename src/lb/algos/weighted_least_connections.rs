use crate::lb::Loadbalancer;
use crate::Server;

pub struct WeightedLeastConnections {}

impl WeightedLeastConnections {
    pub fn new() -> Self {
        WeightedLeastConnections {}
    }
}

impl Loadbalancer for WeightedLeastConnections {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        let min_index = servers
            .iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections / server.weight)
            .map(|(index, _)| index);
        min_index
    }
}
