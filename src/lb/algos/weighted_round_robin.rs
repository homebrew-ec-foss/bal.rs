use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use crate::lb::{servers_alive, LoadBalancer};
use crate::Config;

pub struct WeightedRoundRobin {
    config: Arc<Mutex<Config>>,
    current_index: AtomicUsize,
    current_weight: Mutex<usize>,
}

impl WeightedRoundRobin {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        WeightedRoundRobin {
            config,
            current_index: AtomicUsize::new(0),
            current_weight: Mutex::new(0),
        }
    }

    fn safe_lock<T>(mutex: &Mutex<T>) -> Option<MutexGuard<T>> {
        mutex.lock().ok()
    }
}

impl LoadBalancer for WeightedRoundRobin {
    fn get_server(&self) -> Option<u32> {
        let config = WeightedRoundRobin::safe_lock(&self.config)?;
        while servers_alive(&config.alive) {
            let index = self.current_index.fetch_add(1, Ordering::SeqCst) % config.servers.len();

            let mut weight = WeightedRoundRobin::safe_lock(&self.current_weight)?;
            if *weight >= config.weights[index] as usize {
                *weight = 0;
                continue;
            }

            if config.alive[index] {
                *weight += 1;
                return Some(index as u32);
            }
        }
        None
    }
}