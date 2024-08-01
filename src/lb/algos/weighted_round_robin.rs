use crate::lb::Loadbalancer;
use crate::Server;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct WeightedRoundRobin {
    counter: AtomicUsize,
    current_weight: AtomicUsize,
}

impl WeightedRoundRobin {
    pub fn new() -> Self {
        WeightedRoundRobin {
            counter: AtomicUsize::new(0),
            current_weight: AtomicUsize::new(0),
        }
    }

    fn gcd(a: usize, b: usize) -> usize {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }
}

impl Loadbalancer for WeightedRoundRobin {
    fn get_index(&mut self, servers: &[&Server]) -> Option<usize> {
        let weights: Vec<usize> = servers.iter().map(|s| s.weight as usize).collect();
        let max_weight = *weights.iter().max().unwrap_or(&1);
        let gcd_weight = weights.iter().copied().reduce(Self::gcd).unwrap_or(1);
        let len = servers.len();
        if len == 0 {
            return None;
        }

        loop {
            let index = self.counter.fetch_add(1, Ordering::SeqCst) % len;
            if index == 0 {
                let mut current_weight = self.current_weight.load(Ordering::SeqCst);
                current_weight = if current_weight == 0 {
                    max_weight
                } else {
                    current_weight - gcd_weight
                };
                self.current_weight.store(current_weight, Ordering::SeqCst);
            }

            if servers[index].weight as usize >= self.current_weight.load(Ordering::SeqCst) {
                return Some(index);
            }
        }
    }
}
