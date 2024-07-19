use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::lb::LoadBalancer;

pub struct weighted_round_robin {
    backends: Arc<Vec<(hyper::Uri, u32)>>,
    weights: Arc<Vec<u32>>,
    current_index: AtomicUsize,
    current_weight: AtomicUsize,
    max_weight: usize,
    gcd_weight: usize,
}

impl weighted_round_robin {
    pub fn new(backends: Vec<(hyper::Uri)>, weights: Vec<u32>) -> Self {
        let max_weight = (weights.iter().cloned().fold(0, u32::max) * 100) as usize;
        let gcd_weight = Self::calculate_gcd(&weights);

        weighted_round_robin {
            backends: Arc::new(backends.into_iter()
                                .zip(weights.iter())
                                .map(|(backend, &weight)| (backend, weight))
                                .collect()),
            weights: Arc::new(weights),
            current_index: AtomicUsize::new(0),
            current_weight: AtomicUsize::new(max_weight),
            max_weight,
            gcd_weight,
        }
    }

    fn calculate_gcd(weights: &Vec<u32>) -> usize {
        weights
            .iter()
            .map(|&weight| (weight * 100) as usize)
            .fold(0, |acc, x| Self::gcd(acc, x))
    }

    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 { a } else { Self::gcd(b, a % b) }
    }
}

impl LoadBalancer for weighted_round_robin {
    fn get_server(&self) -> hyper::Uri {
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

            let weight = (self.weights[new_index] * 100) as usize;
            if self.current_weight.load(Ordering::SeqCst) <= weight {
                return self.backends[new_index].0.clone();
            }
        }
    }
}
