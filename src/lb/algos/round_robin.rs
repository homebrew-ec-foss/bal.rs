use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, MutexGuard};

use crate::lb::LoadBalancer;
use crate::Config;

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

impl LoadBalancer for RoundRobin {
    fn get_index(&mut self, config: Arc<&MutexGuard<Config>>) -> Option<usize> {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % config.servers.len();
        Some(index)
    }
}   