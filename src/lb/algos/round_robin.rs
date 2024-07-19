use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::lb::LoadBalancer;

pub struct RoundRobin {
    backends: Arc<Vec<String>>,
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new(backends: Vec<String>) -> Self {
        RoundRobin {
            backends: Arc::new(backends),
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn get_server(&self) -> String {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % self.backends.len();
        self.backends[index].clone()
    }
}
