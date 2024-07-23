use hyper::Uri;
use tokio::time::Duration;
use crate::lb::LoadBalancer;

pub struct least_response_time {
    pub backends: Vec<Uri>,
    pub response_times: Vec<Duration>,
}

impl least_response_time {
    pub fn new(backends: Vec<hyper::Uri>) -> Self {
        least_response_time {
            response_times: vec![Duration::from_secs(0); backends.len()],
            backends: backends,
        }
    }
    pub fn update(&mut self, response_times: Vec<Duration>) {
        self.response_times = response_times;
    }
}

impl LoadBalancer for least_response_time {
    fn get_server(&mut self) -> Uri {
        let min_index = self.response_times.iter()
                            .enumerate()
                            .min_by_key(|(_, (response_time))| response_time.as_millis() as u32) // (index, response_time)
                            .unwrap_or((0, &self.response_times[0])); // Default to index 0 if no servers are available

        self.backends.get(min_index.0).unwrap().clone()
    }
}