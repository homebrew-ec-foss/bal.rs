use crate::lb::LoadBalancer;

pub struct LeastConnections {

}

impl LeastConnections {
    pub fn new() -> Self {
        LeastConnections {}
    }
}

impl LoadBalancer for LeastConnections {
    fn get_index(&mut self, config: std::sync::Arc<&std::sync::MutexGuard<crate::Config>>) -> Option<usize> {
        let min_index = config.servers.iter()
            .enumerate()
            .min_by_key(|(_, server)| server.connections);
        match min_index {
            Some((index, _)) => Some(index),
            None => None
        }
    }
}