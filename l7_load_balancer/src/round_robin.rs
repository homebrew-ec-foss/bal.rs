use std::sync::atomic::{AtomicUsize, Ordering};

static INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn get_next_backend_port(ports: &Vec<u16>) -> u16 {
    let index = INDEX.fetch_add(1, Ordering::Relaxed) % ports.len();
    ports[index]
}