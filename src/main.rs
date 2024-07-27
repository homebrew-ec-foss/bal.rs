use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::error::Error;
use std::time::Duration;
use std::{env, process};

mod lb;

#[derive(Debug)]
struct Config {
    load_balancer: hyper::Uri,
    algo: Algorithm,
    servers: Vec<Server>,
    timeout: Duration,
    max_retries: u32,
    health_check_interval: Duration,
    dead_servers: Vec<Server>,
}

#[derive(Debug, Clone, PartialEq)]
struct Server {
    addr: hyper::Uri,
    weight: u32,
    response_time: Duration,
    connections: u32,
    max_connections: u32
}

impl Server {
    fn new(addr: hyper::Uri, weight: u32, max_connections: u32) -> Self {
        Server {
            addr,
            weight,
            max_connections,
            response_time: Duration::from_secs(0),
            connections: 0,
        }
    }
}

impl Config {
    fn new() -> Self {
        Config {
            load_balancer: "http://127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer 
            algo: Algorithm::RoundRobin, // using round robin as default algorithm
            servers: Vec::new(),
            timeout: Duration::from_secs(0),
            max_retries: 0,
            health_check_interval: Duration::from_secs(0),
            dead_servers: Vec::new(),
        }
    }
    fn update(&mut self, path: &str) -> io::Result<&Config> {
        let path = Path::new(path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut servers: Vec<hyper::Uri> = Vec::new();
        let mut weights: Vec<u32> = Vec::new();
        let mut max_connections: Vec<u32> = Vec::new();

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
                let servers_str = line.trim_start_matches("servers:").trim();
                servers = servers_str
                                    .split(",")
                                    .map(|server| server
                                        .trim()
                                        .parse::<hyper::Uri>()
                                        .expect("Invalid URI"))
                                    .collect();

            } else if line.starts_with("weights:") {
                let weights_str = line.trim_start_matches("weights:").trim();
                weights = weights_str
                                    .split(",")
                                    .map(|weight| weight
                                        .trim()
                                        .parse::<u32>()
                                        .expect("Invalid weight"))
                                    .collect();
                // println!("{:?}", weights);
                
            } else if line.starts_with("max connections:") {
                let max_connections_str = line.trim_start_matches("max connections:").trim();
                max_connections = max_connections_str
                                        .split(",")
                                        .map(|max_connection| max_connection
                                                .trim()
                                                .parse::<u32>()
                                                .expect("Invalid max connection"))
                                        .collect();
                
            } else if line.starts_with("timeout:") {
                let timeout = line.trim_start_matches("timeout:").trim();
                self.timeout = Duration::from_secs(timeout.parse::<u64>().expect("Invalid timeout"));
                
            } else if line.starts_with("max retries:") {
                let max_retries = line.trim_start_matches("max retries:").trim();
                self.max_retries = max_retries.parse::<u32>().expect("Invalid timeout");

            } else if line.starts_with("health check interval:") {
                let health_check_interval = line.trim_start_matches("health check interval:").trim();
                self.health_check_interval = Duration::from_secs(health_check_interval.parse::<u64>().expect("Invalid helth check interval"));
            }
        }
        
        for i in 0..servers.len() {
            self.servers.push(Server::new(servers[i].clone(), weights[i], max_connections[i]));
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
    let mut config = Config::new();
    config.update("config.yaml")?;

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match run(args, &mut config) {
            true => drop(lb::start_lb(config)),
            false => process::exit(0)
        }
    } else {
        println!(r#" ________  ___  ________  ________  ___  ___  ________      "#);
        println!(r#"|\   ____\|\  \|\   __  \|\   ____\|\  \|\  \|\   ____\     "#);
        println!(r#"\ \  \___|\ \  \ \  \|\  \ \  \___|\ \  \\\  \ \  \___|_    "#);
        println!(r#" \ \  \    \ \  \ \   _  _\ \  \    \ \  \\\  \ \_____  \   "#);
        println!(r#"  \ \  \____\ \  \ \  \\  \\ \  \____\ \  \\\  \|____|\  \  "#);
        println!(r#"   \ \_______\ \__\ \__\\ _\\ \_______\ \_______\____\_\  \ "#);
        println!(r#"    \|_______|\|__|\|__|\|__|\|_______|\|_______|\_________\"#);
        println!("Type h or ? for a list of available commands");

        let usn = whoami::username() + &String::from("@circus");

        let mut cli_completed = false;
        while !cli_completed {
            let mut arg = String::new();

            print!("{usn}:> ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut arg).unwrap();

            let mut args: Vec<String> = arg.trim().split_whitespace().map(String::from).collect();
            args.insert(0, "Blank".to_string());

            cli_completed = run(args, &mut config);
        }
        drop(lb::start_lb(config));
    }

    Ok(())
}

fn get_algo(algo: &str) -> Algorithm {
    match algo {
        "round_robin" => Algorithm::RoundRobin,
        "weighted_round_robin" => Algorithm::WeightedRoundRobin,
        "least_connections" => Algorithm::LeastConnections,
        "weighted_least_connections" => Algorithm::WeightedLeastConnections,
        "least_response_time" => Algorithm::LeastResponseTime,
        "weighted_least_response_time" => Algorithm::WeightedLeastResponseTime,
        _ => Algorithm::RoundRobin, // Default algorithms
    }
}

fn run(args: Vec<String>, config: &mut Config) -> bool {  
    match args[1].as_str() {
        "h" | "?" => {
            help();
            false
        },
        "start" => true,
        //"stop" => lb::stop_lb(config).unwrap(), //Implement later
        "p" => {
            let _p: u32 = match args[2].trim().parse() {
                Ok(prt) => prt,
                Err(e) => {
                    println!("Invalid argument passed as port number: {:?}", e);
                    return false;
                }
            };
            //lb::change_port(p); //Implement later
            false
        },
        "a" => {
            let _a = get_algo(args[2].trim()); //Fully implement later
            config.algo = _a;

            false
        },
        "s" => {
            println!("{} servers available", config.servers.len());

            false
        },
        "q" => {
            println!("Exiting..");
            process::exit(0);
        },
        _ => {
            println!("Unknown argument passed");
            false
        }
    }
}

fn help() {
    println!("h or ? -> Displays this list of available commands");
    println!("q -> Quit program. Applicable only when program is run with no arguments");
    println!("start -> Starts the load balancer");
    //println!("stop -> Stops the load balancer");
    println!("p <port_number> -> Changes load balancer port to specified port. Takes one more argument as port number");
    println!("a <algorithm> -> Changes load balancer algorithm to specified algorithm. Takes one more argument as algorithm name");
    println!("s -> Shows number of available servers");
}