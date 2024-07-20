use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Uri};
use hyper::http::StatusCode;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::error::Error;
use tokio::time::{sleep, Duration, timeout};

mod algos;
use algos::round_robin::round_robin;
use algos::weighted_round_robin::weighted_round_robin;
//use algos::least_connections::least_connections;
use algos::weighted_least_connections::WeightedLeastConnections;
use algos::least_response_time::least_response_time;
use algos::weighted_least_response_time::weighted_least_response_time;

use crate::Config;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    // Ensure the URI has an authority part (host and port)
    let authority = uri.authority().ok_or("URI does not have an authority part")?;
    
    // Extract host and port
    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;
    
    // Combine host and port into a SocketAddr
    let addr_str = format!("{}:{}", host, port);
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

#[tokio::main]
pub async fn start_lb(config: Arc<Config>) -> Result<(), Box<dyn Error>> {
    let addr = uri_to_socket_addr(&config.load_balancer)?;

    match config.algo { // im just using round robin for all now, will change once algos are done
        crate::Algorithm::round_robin => {
            start_server(addr, Arc::new(Mutex::new(round_robin::new(config.servers.clone())))).await;
        },
        crate::Algorithm::weighted_round_robin => {
            start_server(addr, Arc::new(Mutex::new(weighted_round_robin::new(config.servers.clone(), config.weights.clone())))).await;
        },
        crate::Algorithm::least_connections => {
            start_server(addr, Arc::new(Mutex::new(round_robin::new(config.servers.clone())))).await; // least_connections::new(&config.servers),
        }, 
        crate::Algorithm::weighted_least_connections => {
            start_server(addr, Arc::new(Mutex::new(WeightedLeastConnections::new(config.servers.clone(), config.weights.clone(), vec![100; config.weights.len()])))).await; // weighted_least_connections::new(&config.servers, &config.weights),
        },
        crate::Algorithm::least_response_time => {
            let load_balancer = Arc::new(Mutex::new(least_response_time::new(config.servers.clone())));

            let load_balancer_clone = Arc::clone(&load_balancer);
            let config_clone = Arc::clone(&config);

            tokio::spawn(async move { // loop to update response time
                loop {
                    let mut response_times = vec![];
                    let client = Client::new();
                    
                    for server in &config_clone.servers {
                        let start = std::time::Instant::now();

                        let response_time = match timeout(Duration::from_secs(1), client.get(format!("http://{}", server.clone()).parse::<Uri>().unwrap())).await {
                            Ok(Ok(_)) => start.elapsed(),
                            err => {
                                eprintln!("{:?}", err);
                                Duration::from_secs(u64::MAX)
                            }
                        };

                        response_times.push(response_time);
                    }
                    
                    {
                        let mut load_balancer_clone = load_balancer_clone.lock().unwrap();
                        load_balancer_clone.update(response_times);
                    }
                    sleep(Duration::from_secs(30)).await; // updates response time every 30 seconds
                }
            });

            start_server(addr, load_balancer).await;
        },
        crate::Algorithm::weighted_least_response_time => {
            let load_balancer = Arc::new(Mutex::new(weighted_least_response_time::new(config.servers.clone(), config.weights.clone())));

            let load_balancer_clone = Arc::clone(&load_balancer);
            let config_clone = Arc::clone(&config);

            tokio::spawn(async move { // loop to update response time
                loop {
                    let mut response_times = vec![];
                    let client = Client::new();
                    
                    for server in &config_clone.servers {
                        let start = std::time::Instant::now();

                        let response_time = match timeout(Duration::from_secs(1), client.get(format!("http://{}", server.clone()).parse::<Uri>().unwrap())).await {
                            Ok(Ok(_)) => start.elapsed(),
                            err => {
                                eprintln!("{:?}", err);
                                Duration::from_secs(u64::MAX)
                            }
                        };

                        response_times.push(response_time);
                    }
                    
                    {
                        let mut load_balancer_clone = load_balancer_clone.lock().unwrap();
                        load_balancer_clone.update(response_times);
                    }
                    sleep(Duration::from_secs(30)).await; // updates response time every 30 seconds
                }
            });

            start_server(addr, load_balancer).await;
        },
    };
    
    Ok(())
}

async fn start_server<T>(addr: SocketAddr, load_balancer: Arc<Mutex<T>>)
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
    load_balancer: Arc<Mutex<T>>,
) -> Result<Response<Body>, hyper::Error>
where
    T: LoadBalancer + Send + Sync + 'static,
{   
    let uri = load_balancer.lock().unwrap().get_server();
    let uri = format!("http://{}", uri);
    let uri: Uri = uri.parse().unwrap();
    println!("{:?}", uri);
    let client = Client::new();

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

pub trait LoadBalancer: Send + Sync {
    fn get_server(&mut self) -> hyper::Uri;
}