use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use crate::lb::{servers_alive, LoadBalancer};
use crate::Config;

pub struct WeightedRoundRobin {
    config: Arc<Mutex<Config>>,
    current_index: AtomicUsize,
    current_weight: AtomicUsize,
    max_weight: usize,
    gcd_weight: usize,
}

impl WeightedRoundRobin {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
	let binding=config.clone();
        let config_lock = binding.lock().unwrap();
        let weights: Vec<f64> = config_lock.weights.iter().map(|&w| w as f64).collect();
        let max_weight = (weights.iter().cloned().fold(0./0., f64::max) * 100.0) as usize;
        let gcd_weight = Self::calculate_gcd(&weights);
        
        WeightedRoundRobin {
            config,
            current_index: AtomicUsize::new(0),
            current_weight: AtomicUsize::new(max_weight),
            max_weight,
            gcd_weight,
        }
    }

    fn calculate_gcd(weights: &Vec<f64>) -> usize {
        weights
            .iter()
            .map(|&weight| (weight * 100.0).round() as usize)
            .fold(0, |acc, x| Self::gcd(acc, x))
    }

    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 { a } else { Self::gcd(b, a % b) }
    }
}

impl LoadBalancer for WeightedRoundRobin {
    fn get_server(&self) -> Option<u32> {
        loop {
            let config = self.config.lock().unwrap();
            if !servers_alive(&config.alive) {
                return None;
            }

            let current_index = self.current_index.load(Ordering::SeqCst);
            let new_index = (current_index + 1) % config.servers.len();
            self.current_index.store(new_index, Ordering::SeqCst);

            if new_index == 0 {
                let current_weight = self.current_weight.load(Ordering::SeqCst);
                let new_weight = current_weight.saturating_sub(self.gcd_weight);
                self.current_weight.store(new_weight, Ordering::SeqCst);

                if new_weight <= 0 {
                    self.current_weight.store(self.max_weight, Ordering::SeqCst);
                }
            }

            let weight = (config.weights[new_index] as f64 * 100.0).round() as usize;
            if self.current_weight.load(Ordering::SeqCst) <= weight {
                if config.alive[new_index] {
                    return Some(new_index as u32);
                }
            }
        }
    }
}
