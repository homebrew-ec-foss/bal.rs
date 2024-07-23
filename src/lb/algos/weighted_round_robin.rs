use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use crate::lb::{servers_alive, LoadBalancer};
use crate::Config;

pub struct WeightedRoundRobin {
    config: Arc<Mutex<Config>>,
    current_index: AtomicUsize,
    current_weight: usize,
}

impl WeightedRoundRobin {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        WeightedRoundRobin {
            config,
            current_index: AtomicUsize::new(0),
            current_weight: 0,
        }
    }
}

impl LoadBalancer for WeightedRoundRobin {
    fn get_server(&mut self) -> Option<u32> {
        while servers_alive(&self.config.lock().unwrap().alive) {
            let index = self.current_index.fetch_add(1, Ordering::SeqCst) % self.config.lock().unwrap().servers.len();

            if self.current_weight >= self.config.lock().unwrap().weights[index] as usize {
                self.current_weight = 0;
                continue;
            }

            if self.config.lock().unwrap().alive[index] {
                self.current_weight += 1;
                return Some(index as u32);
            }
        }
        None
    }
}
