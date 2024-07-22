use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use crate::lb::LoadBalancer;
use crate::Config;

pub struct RoundRobin {
    config: Arc<Mutex<Config>>,
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        RoundRobin {
            config: config,
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn get_server(&self) -> hyper::Uri {
        let index = self.counter.fetch_add(1, Ordering::SeqCst) % self.config.lock().unwrap().servers.len();
        self.config.lock().unwrap().servers[index].clone()
    }
}