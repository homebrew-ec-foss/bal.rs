use std::sync::atomic::{AtomicUsize, Ordering};

use crate::lb::Loadbalancer;
use crate::Server;

pub struct RoundRobin {
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        RoundRobin {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Loadbalancer for RoundRobin {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % servers.len();
        Some(index)
    }
}
