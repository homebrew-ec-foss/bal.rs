use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::LoadBalancer;

pub struct WeightedLoadBalancer {
    backends: Arc<Vec<(String, f64)>>,
    weights: Arc<Vec<f64>>,
    current_index: AtomicUsize,
    current_weight: AtomicUsize,
    max_weight: usize,
    gcd_weight: usize,
}

impl WeightedLoadBalancer {
    pub fn new(backends: Vec<(String, f64)>) -> Self {
        let weights: Vec<f64> = backends.iter().map(|&(_, weight)| weight).collect();
        let max_weight = (weights.iter().cloned().fold(0./0., f64::max) * 100.0) as usize;
        let gcd_weight = Self::calculate_gcd(&weights);

        WeightedLoadBalancer {
            backends: Arc::new(backends),
            weights: Arc::new(weights),
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

impl LoadBalancer for WeightedLoadBalancer {
    fn get_next_backend(&self) -> String {
        loop {
            let current_index = self.current_index.load(Ordering::SeqCst);
            let new_index = (current_index + 1) % self.backends.len();
            self.current_index.store(new_index, Ordering::SeqCst);

            if new_index == 0 {
                let current_weight = self.current_weight.load(Ordering::SeqCst);
                let new_weight = current_weight - self.gcd_weight;
                self.current_weight.store(new_weight, Ordering::SeqCst);

                if new_weight <= 0 {
                    self.current_weight.store(self.max_weight, Ordering::SeqCst);
                }
            }

            let weight = (self.weights[new_index] * 100.0).round() as usize;
            if self.current_weight.load(Ordering::SeqCst) <= weight {
                return self.backends[new_index].0.clone();
            }
        }
    }
}
