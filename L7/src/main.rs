use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Uri};
use hyper::http::StatusCode;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

mod lib;

use lib::round_robin::RoundRobin;

use lib::weighted_load_balancer::WeightedLoadBalancer;

use server;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let backend = server::get_uris("server/servers.csv".to_string()).ok().unwrap();

    let mut backends = Vec::new();

    for uri in backend {
        backends.push(format!("http://{}", uri.to_string()));
    }

    loop {
        println!("Type 's' to start the server or 'q' to quit:");

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        match input.trim() {
            "s" => {
                println!("Choose an option:\n1. Start server with round-robin algorithm\n2. Start server with weighted load balancing\n3. Terminate");

                input.clear();
                io::stdin().read_line(&mut input).expect("Failed to read input");

                match input.trim() {
                    "1" => {
                        println!("Starting server with round-robin algorithm...");

                        let round_robin = Arc::new(RoundRobin::new(backends.clone()));
                        start_server(addr, round_robin).await;
                    },
                    "2" => {
                        let weights = get_weights(&backends);
                        let weighted_backends = backends.iter().zip(weights.iter()).map(|(b, &w)| (b.clone(), w)).collect();

                        println!("Starting server with weighted load balancing...");

                        let weighted_load_balancer = Arc::new(WeightedLoadBalancer::new(weighted_backends));
                        start_server(addr, weighted_load_balancer).await;
                    },
                    "3" => {
                        println!("Terminating...");
                        return;
                    },
                    _ => {
                        println!("Invalid option. Please try again.");
                    }
                }
            },
            "q" => {
                println!("Quitting...");
                return;
            },
            _ => {
                println!("Invalid input. Please type 's' to start the server or 'q' to quit.");
            }
        }
    }
}

fn get_weights(backends: &Vec<String>) -> Vec<f64> {
    let mut weights = Vec::new();
    let mut total_weight = 0.0;

    for backend in backends {
        loop {
            println!("Enter weight for {} (remaining weight to allocate: {}):", backend, 1.0 - total_weight);
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read input");
            let weight: f64 = match input.trim().parse() {
                Ok(w) => w,
                Err(_) => {
                    println!("Invalid weight. Please enter a decimal number.");
                    continue;
                }
            };

            if weight <= 0.0 || weight > 1.0 || total_weight + weight > 1.0 {
                println!("Invalid weight. Make sure the weight is positive and the total does not exceed 1.");
            } else {
                weights.push(weight);
                total_weight += weight;
                break;
            }
        }
    }

    if (total_weight - 1.0).abs() > std::f64::EPSILON {
        println!("Weights do not add up to 1. Please restart the server and try again.");
        std::process::exit(1);
    }

    weights
}

async fn start_server<T>(addr: SocketAddr, load_balancer: Arc<T>)
where
    T: LoadBalancer + Send + Sync + 'static,
{
    let make_svc = make_service_fn(move |_| {
        let load_balancer = load_balancer.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let load_balancer = load_balancer.clone();
                async move {
                    match forward_request(req, load_balancer).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            eprintln!("Request failed: {}", e);
                            Ok::<_, Infallible>(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from("Internal Server Error"))
                                .unwrap())
                        }
                    }
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

async fn forward_request<T>(
    req: Request<Body>,
    load_balancer: Arc<T>,
) -> Result<Response<Body>, hyper::Error>
where
    T: LoadBalancer + Send + Sync + 'static,
{
    let backend = load_balancer.get_next_backend();

    let client = Client::new();

    let uri_string = format!("{}{}", backend, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
    let uri: Uri = uri_string.parse().unwrap();

    let mut new_req_builder = Request::builder()
        .method(req.method())
        .uri(uri);

    for (key, value) in req.headers().iter() {
        new_req_builder = new_req_builder.header(key, value);
    }

    let new_req = new_req_builder
        .body(req.into_body())
        .unwrap();

    client.request(new_req).await
}

pub trait LoadBalancer {
    fn get_next_backend(&self) -> String;
}
