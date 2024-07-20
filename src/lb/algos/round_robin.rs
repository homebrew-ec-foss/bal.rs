use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::lb::LoadBalancer;

pub struct round_robin {
    backends: Arc<Vec<hyper::Uri>>,
    counter: AtomicUsize,
}

impl round_robin {
    pub fn new(backends: Vec<hyper::Uri>) -> Self {
        round_robin {
            backends: Arc::new(backends),
            counter: AtomicUsize::new(0),
        }
    }
}


impl LoadBalancer for round_robin {
    fn get_server(&mut self) -> hyper::Uri {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % self.backends.len();
        self.backends[index].clone()
    }
}
