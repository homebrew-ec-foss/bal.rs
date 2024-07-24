use std::sync::Arc;
use std::net::IpAddr;
use hyper::Uri;
use crate::lb::LoadBalancer;
use murmur3::murmur3_32;
use std::io::Cursor;

pub struct IpHash {
    backends: Arc<Vec<Uri>>,
}

impl IpHash {
    pub fn new(backends: Vec<Uri>) -> Self {
        IpHash {
            backends: Arc::new(backends),
        }
    }

    fn hash_ip(ip: &str) -> u32 {
        murmur3_32(&mut Cursor::new(ip), 0).unwrap()
    }
}

impl LoadBalancer for IpHash {
    fn get_server(&mut self, client_ip: &str) -> Uri {
        let hash = IpHash::hash_ip(client_ip);
        let index = (hash as usize) % self.backends.len();
        self.backends[index].clone()
    }
}
