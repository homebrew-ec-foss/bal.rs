use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::lb::LoadBalancer;
use crate::{Config, Server};

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
    fn get_index(&mut self, servers: &Vec<Server>) -> Option<usize> {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % servers.len();
        Some(index)
    }
}   