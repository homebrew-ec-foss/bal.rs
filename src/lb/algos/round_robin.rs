use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::MutexGuard;

use crate::lb::Loadbalancer;
use crate::LoadBalancer;

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
    fn get_index(&mut self, lb: &MutexGuard<LoadBalancer>) -> Option<usize> {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % lb.servers.len();
        Some(index)
    }
}
