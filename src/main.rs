use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::error::Error;
use std::time::Duration;

mod lb;

#[derive(Debug)]
struct Config {
    load_balancer: hyper::Uri,
    algo: Algorithm,
    servers: Vec<hyper::Uri>,
    weights: Vec<u32>,
    alive: Vec<bool>,
    response_time: Vec<Duration>,
    connections: Vec<u32>,
    max_connections: Vec<u32>,
    timeout: Duration,
    max_retries: u32,
    health_check_interval: Duration,
}

impl Config {
    fn new() -> Self {
        Config {
            load_balancer: "http://127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer 
            algo: Algorithm::round_robin, // using round robin as default algorithm
            servers: Vec::new(),
            weights: Vec::new(),
            alive: Vec::new(),
            response_time: Vec::new(),
            connections: Vec::new(),
            max_connections: Vec::new(),
            timeout: Duration::from_secs(0),
            max_retries: 0,
            health_check_interval: Duration::from_secs(0),
        }
    }
    fn update(&mut self, path: &str) -> io::Result<&Config> {
        let path = Path::new(path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("load balancer:") {
                let load_balancer = line.trim_start_matches("load balancer:").trim().parse::<hyper::Uri>();
                let load_balancer = match load_balancer {
                    Ok(load_balancer) => load_balancer,
                    Err(_) => "http://127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer
                };
                self.load_balancer = load_balancer;

            } else if line.starts_with("algorithm:") {
                let algo = line.trim_start_matches("algorithm:").trim();
                self.algo = get_algo(algo);

            } else if line.starts_with("servers:") {
                let servers = line.trim_start_matches("servers:").trim();
                self.servers = servers
                                    .split(",")
                                    .map(|server| server
                                        .trim()
                                        .parse::<hyper::Uri>()
                                        .expect("Invalid URI"))
                                    .collect();

            } else if line.starts_with("weights:") {
                let weights = line.trim_start_matches("weights:").trim();
                self.weights = weights
                                    .split(",")
                                    .map(|weight| weight
                                        .trim()
                                        .parse::<u32>()
                                        .expect("Invalid weight"))
                                    .collect();

                self.alive = vec![true; self.servers.len()];
                self.response_time = vec![Duration::from_secs(0); self.servers.len()];
                self.connections = vec![0; self.servers.len()];
                
            } else if line.starts_with("max connections:") {
                let max_connections = line.trim_start_matches("max connections:").trim();
                self.max_connections = max_connections
                                        .split(",")
                                        .map(|max_connection| max_connection
                                                .trim()
                                                .parse::<u32>()
                                                .expect("Invalid max connection"))
                                        .collect();
                
            } else if line.starts_with("timeout:") {
                let timeout = line.trim_start_matches("timeout:").trim();
                self.timeout = Duration::from_millis(timeout.parse::<u64>().expect("Invalid timeout"));
                
            } else if line.starts_with("max retries:") {
                let max_retries = line.trim_start_matches("max retries:").trim();
                self.max_retries = max_retries.parse::<u32>().expect("Invalid timeout");

            } else if line.starts_with("helth check interval:") {
                let health_check_interval = line.trim_start_matches("helth check interval:").trim();
                self.health_check_interval = Duration::from_millis(health_check_interval.parse::<u64>().expect("Invalid helth check interval"));
            }
        }

        Ok(self)
    }
}

#[derive(Debug)]
enum Algorithm {
    round_robin,
    weighted_round_robin,
    least_connections,
    weighted_least_connections,
    least_response_time,
    weighted_least_response_time,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut config = Config::new();
    let ref_config = config.update("config.yaml")?;

    println!("{:?}", ref_config); // remove

    // cli

    lb::start_lb(config);

    Ok(())
}

fn get_algo(algo: &str) -> Algorithm {
    match algo {
        "round_robin" => Algorithm::round_robin,
        "weighted_round_robin" => Algorithm::weighted_round_robin,
        "least_connections" => Algorithm::least_connections,
        "weighted_least_connections" => Algorithm::weighted_least_connections,
        "least_response_time" => Algorithm::least_response_time,
        "weighted_least_response_time" => Algorithm::weighted_least_response_time,
        _ => Algorithm::round_robin, // Default algorithms
    }
}