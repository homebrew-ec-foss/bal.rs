use clap::{command, Arg, ArgAction, Command};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

mod lb;

#[derive(Debug)]
struct LoadBalancer {
    load_balancer: hyper::Uri,
    algo: Algorithm,
    servers: Vec<Server>,
    timeout: Duration,
    health_check_interval: Duration,
    report: bool,
    save_file: String,
    save: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Server {
    addr: hyper::Uri,
    weight: u32,
    response_time: Duration,
    connections: u32,
    max_connections: u32,
    alive: bool,
    connections_served: u32,
}

impl Server {
    fn new(addr: hyper::Uri, weight: u32, max_connections: u32) -> Self {
        Server {
            addr,
            weight,
            max_connections,
            response_time: Duration::from_secs(0),
            connections: 0,
            alive: true,
            connections_served: 0,
        }
    }
}

impl LoadBalancer {
    fn new() -> Self {
        LoadBalancer {
            load_balancer: "http://127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer
            algo: Algorithm::RoundRobin, // using round robin as default algorithm
            servers: Vec::new(),
            timeout: Duration::from_secs(0),
            health_check_interval: Duration::from_secs(0),
            report: true,
            save_file: String::from("data.txt"),
            save: false,
        }
    }
    fn update(&mut self, path: &str) -> io::Result<&LoadBalancer> {
        let path = Path::new(path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut servers: Vec<hyper::Uri> = Vec::new();
        let mut weights: Vec<u32> = Vec::new();
        let mut max_connections: Vec<u32> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("load balancer:") {
                let load_balancer = line
                    .trim_start_matches("load balancer:")
                    .trim()
                    .parse::<hyper::Uri>();
                let load_balancer = match load_balancer {
                    Ok(load_balancer) => load_balancer,
                    Err(_) => "http://127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer
                };
                self.load_balancer = load_balancer;
            } else if line.starts_with("algorithm:") {
                let algo = line.trim_start_matches("algorithm:").trim();
                self.algo = get_algo(algo);
            } else if line.starts_with("servers:") {
                let servers_str = line.trim_start_matches("servers:").trim();
                servers = servers_str
                    .split(',')
                    .map(|server| server.trim().parse::<hyper::Uri>().expect("Invalid URI"))
                    .collect();
            } else if line.starts_with("weights:") {
                let weights_str = line.trim_start_matches("weights:").trim();
                weights = weights_str
                    .split(',')
                    .map(|weight| weight.trim().parse::<u32>().expect("Invalid weight"))
                    .collect();
            } else if line.starts_with("max connections:") {
                let max_connections_str = line.trim_start_matches("max connections:").trim();
                max_connections = max_connections_str
                    .split(',')
                    .map(|max_connection| {
                        max_connection
                            .trim()
                            .parse::<u32>()
                            .expect("Invalid max connection")
                    })
                    .collect();
            } else if line.starts_with("timeout:") {
                let timeout = line.trim_start_matches("timeout:").trim();
                self.timeout =
                    Duration::from_secs(timeout.parse::<u64>().expect("Invalid timeout"));
            } else if line.starts_with("health check interval:") {
                let health_check_interval =
                    line.trim_start_matches("health check interval:").trim();
                self.health_check_interval = Duration::from_secs(
                    health_check_interval
                        .parse::<u64>()
                        .expect("Invalid health check interval"),
                );
            }
        }

        for i in 0..servers.len() {
            self.servers.push(Server::new(
                servers[i].clone(),
                weights[i],
                max_connections[i],
            ));
        }

        Ok(self)
    }
}

#[derive(Debug, Clone)]
enum Algorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    WeightedLeastConnections,
    LeastResponseTime,
    WeightedLeastResponseTime,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut lb = LoadBalancer::new();
    lb.update("config.yaml")?;

    cli(lb)
    // drop(lb::start_lb(lb));
    // Ok(())
}

fn cli(mut lb: LoadBalancer) -> Result<(), Box<dyn Error>> {
    let res = command!()
        .arg_required_else_help(true)
        .about(
            r#"
 ________  ________  ___           ________  ________          
|\   __  \|\   __  \|\  \         |\   __  \|\   ____\         
\ \  \|\ /\ \  \|\  \ \  \        \ \  \|\  \ \  \___|_        
 \ \   __  \ \   __  \ \  \        \ \   _  _\ \_____  \       
  \ \  \|\  \ \  \ \  \ \  \____  __\ \  \\  \\|____|\  \      
   \ \_______\ \__\ \__\ \_______\\__\ \__\\ _\ ____\_\  \     
    \|_______|\|__|\|__|\|_______\|__|\|__|\|__|\_________\    
                                               \|_________|   
L7 Load Balancer Implemented in Rust 🦀
    "#,
        )
        .subcommand(
            Command::new("start")
                .about("Start the load balancer")
                .arg(
                    Arg::new("address")
                        .short('u')
                        .long("address")
                        .help("Starts load balancer at specified address"),
                )
                .arg(Arg::new("algorithm").short('a').long("algorithm").help(
                    "Starts load balancer with specified algorithm.
Available algorithms: round_robin/rr, weighted_round_robin/wrr, least_connections/lc, 
weighted_least_connections/wlc, least_response_time/lrt, weighted_least_response_time/wlrc",
                ))
                .arg(
                    Arg::new("path")
                        .short('p')
                        .long("path")
                        .default_value("config.yaml")
                        .help("Specify path to config file"),
                )
                .arg(
                    Arg::new("report")
                        .short('r')
                        .long("report")
                        .action(ArgAction::SetTrue)
                        .help("Prints server status"),
                )
                .arg(
                    Arg::new("save file")
                        .short('s')
                        .long("save")
                        .help("Saves report data to specified file"),
                ),
        )
        .get_matches();

    match res.subcommand_name() {
        Some("start") => {
            println!("Starting load balancer");
            let start_args = res.subcommand_matches("start").unwrap();
            let path = start_args.get_one::<String>("path");
            let address = start_args.get_one::<String>("address");
            let algorithm = start_args.get_one::<String>("algorithm");
            let report = start_args.get_one::<bool>("report");
            let save = start_args.get_one::<String>("save file");

            if let Some(path) = path {
                lb.servers = Vec::new();
                lb.update(path).unwrap();
            }

            if let Some(address) = address {
                lb.load_balancer = address.trim().parse::<hyper::Uri>().unwrap();
            }

            if let Some(algorithm) = algorithm {
                lb.algo = get_algo(algorithm);
            }

            if let Some(report) = report {
                lb.report = *report;
            }

            if let Some(save) = save {
                lb.save_file.clone_from(save);
                lb.save = true;
            }

            drop(lb::start_lb(lb));
        }
        _ => println!("Invalid command"),
    }

    Ok(())
}

fn get_algo(algo: &str) -> Algorithm {
    match algo {
        // "round_robin" => Algorithm::RoundRobin,
        "weighted_round_robin" | "wrr" => Algorithm::WeightedRoundRobin,
        "least_connections" | "lc" => Algorithm::LeastConnections,
        "weighted_least_connections" | "wlc" => Algorithm::WeightedLeastConnections,
        "least_response_time" | "lrt" => Algorithm::LeastResponseTime,
        "weighted_least_response_time" | "wlrt" => Algorithm::WeightedLeastResponseTime,
        _ => Algorithm::RoundRobin, // Default algorithms
    }
}
