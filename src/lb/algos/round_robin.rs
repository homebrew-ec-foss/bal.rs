use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use crate::lb::{servers_alive, LoadBalancer};
use crate::Config;

pub struct RoundRobin {
    config: Arc<Mutex<Config>>,
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        RoundRobin {
            config,
            counter: AtomicUsize::new(0),
        }
    }
    
}

impl LoadBalancer for RoundRobin {
    fn get_server(&self) -> Option<u32> {
        while servers_alive(&self.config.lock().unwrap().alive) {
            let index = self.counter.fetch_add(1, Ordering::SeqCst) % self.config.lock().unwrap().servers.len();
            if self.config.lock().unwrap().alive[index] {
                return Some(index as u32);
            }
        }
        return None;
    }
}