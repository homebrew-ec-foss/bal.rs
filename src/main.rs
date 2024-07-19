use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::error::Error;
use std::{env, process};

mod lb;

#[derive(Debug)]
struct Config {
    load_balancer: hyper::Uri,
    servers: Vec<hyper::Uri>,
    weights: Vec<u32>,
    algo: Algorithm,
}

impl Config {
    fn new() -> Self {
        Config {
            load_balancer: "127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer 
            servers: Vec::new(),
            weights: Vec::new(),
            algo: Algorithm::round_robin, // using round robin as default algorithm
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
                    Err(_) => "127.0.0.1:8000".parse::<hyper::Uri>().unwrap(), // default address for load balancer
                };
                self.load_balancer = load_balancer;
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
            } else if line.starts_with("algorithm:") {
                let algo = line.trim_start_matches("algorithm:").trim();
                self.algo = match algo {
                    "round_robin" => Algorithm::round_robin,
                    "weighted_round_robin" => Algorithm::weighted_round_robin,
                    "least_connections" => Algorithm::least_connections,
                    "weighted_least_connections" => Algorithm::weighted_least_connections,
                    "least_response_time" => Algorithm::least_response_time,
                    "weighted_least_response_time" => Algorithm::weighted_least_response_time,
                    _ => Algorithm::round_robin, // Default algorithms
                }
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
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        run(args);
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

        loop {
            let mut arg = String::new();

            print!("{usn}:> ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut arg).unwrap();

            let mut args: Vec<String> = arg.trim().split_whitespace().map(String::from).collect();
            args.insert(0, "Blank".to_string());

            run(args);
        }
    }

    Ok(())
}

fn run(args: Vec<String>) {
    let mut config = Config::new();
    let ref_config = config.update("src/.config").unwrap();

    match args[1].as_str() {
        "h" | "?" => help(),
        "start" => lb::start_lb(ref_config).unwrap(),
        //"stop" => lb::stop_lb(ref_config).unwrap(),
        "p" => {
            let p: u32 = match args[2].trim().parse() {
                Ok(prt) => prt,
                Err(_) => {
                    println!("Error: Invalid argument passed as port number");
                    process::exit(0); //Have to change to rerun instead of exiting
                }
            };
            //lb::change_ports(p);
        },
        "c" => {
            let s_count = ref_config.servers.len();
            println!("{s_count} servers available");
        },
        "q" => {
            println!("Exiting..");
            process::exit(0);
        },
        _ => println!("Unknown argument passed")
    }
}

fn help() {
    println!("h or ? -> Displays this list of available commands");
    println!("q -> Quit program. Applicable only when program is run with no arguments");
    println!("start -> Starts the load balancer");
    println!("stop -> Stops the load balancer");
    println!("p <port_number> -> Changes to specified port. Takes one more argument as port number");
    println!("c -> Shows number of available servers");
}