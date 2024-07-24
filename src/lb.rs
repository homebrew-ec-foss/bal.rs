use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Uri, Response};
use hyper_util::rt::TokioIo;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;

use crate::{Algorithm, Config};
mod algos;
use algos::round_robin::RoundRobin;
use algos::static_lb::StaticLB;
use algos::weighted_round_robin::WeightedRoundRobin;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    // takes Uri and returns SocketAddr
    let authority = uri
        .authority()
        .ok_or("URI does not have an authority part")?; // Ensure the URI has an authority part (host and port)

    let host = authority.host(); // Extract host and port
    let port = authority.port_u16().ok_or("URI does not have a port")?;

    let addr_str = format!("{}:{}", host, port); // Combine host and port into a SocketAddr
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

fn servers_alive(status: &Vec<bool>) -> bool {
    for &value in status {
        if value {
            return true;
        }
    }
    false
}

#[tokio::main]
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // starts the load balancer
    let config = Arc::new(Mutex::new(config));

    let static_lb = Arc::new(Mutex::new(StaticLB::new()));
    let len = config.lock().unwrap().servers.len();
    let health_check_interval = config.lock().unwrap().health_check_interval.clone();
    let config_clone = Arc::clone(&config);

    tokio::task::spawn(async move {
        loop {
            let mut tasks = Vec::new();
            for i in 0..len {
                let static_lb_clone = Arc::clone(&static_lb);
                let config_clone_ = Arc::clone(&config_clone);
                let task = tokio::task::spawn(async move {
                    static_lb_clone.lock().unwrap().update(i as u32);
                    drop(
                        handle_request(
                            Arc::new(None),
                            Arc::clone(&config_clone_),
                            Arc::clone(&static_lb_clone),
                            true,
                        )
                        .await,
                    );
                });
                
                sleep(Duration::from_millis(10)).await;
                tasks.push(task);
            }

            for task in tasks {
                drop(task.await);
            }

            println!("updated config {:?}", &config_clone);

            sleep(health_check_interval).await;
        }
    });

    let algo = { config.lock().unwrap().algo.clone() };

    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new(Arc::clone(&config))));
            drop(listen(Arc::clone(&config), load_balancer).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new(Arc::clone(&config))));
            drop(listen(Arc::clone(&config), load_balancer).await);
        }

        Algorithm::LeastConnections => {}
        Algorithm::WeightedLeastConnections => {}
        Algorithm::LeastResponseTime => {}
        Algorithm::WeightedLeastResponseTime => {}
    }

    Ok(())
}

async fn listen<T>(
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: LoadBalancer + Send + 'static,
{
    let addr = uri_to_socket_addr(&config.lock().unwrap().load_balancer).unwrap();

    println!("Server is running on http://{}", addr);

    let listener = TcpListener::bind(addr).await?; // We create a TcpListener and bind it to load balancer address

    loop {
        // starting a loop to continuously accept incoming connections
        let (stream, _) = listener.accept().await?;
        let client_addr = stream.peer_addr()?;
        println!(
            "Accepted connection from {} to load balancer {}",
            client_addr, addr
        );

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let config_clone = Arc::clone(&config);
        let load_balancer_clone = Arc::clone(&load_balancer);

        // !!!!!!!!!! idk smtg virtual thread thing should go down here
        tokio::task::spawn(async move {
            // spawns a tokio task to server multiple connections concurrently
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        handle_request(
                            Arc::new(Some(req)),
                            Arc::clone(&config_clone),
                            Arc::clone(&load_balancer_clone),
                            false,
                        )
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn handle_request<T>(
    req: Arc<Option<Request<hyper::body::Incoming>>>,
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
    health_check: bool,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    T: LoadBalancer,
{
    loop {
        if servers_alive(&config.lock().unwrap().alive) | health_check {
            match get_request(
                Arc::clone(&req),
                Arc::clone(&config),
                Arc::clone(&load_balancer),
            )
            .await
            {
                Some(request) => {
                    return request;
                }
                None => {
                    println!("running again");
                }
            }
            if health_check {
                let response = Response::builder()
                    .status(500)
                    .body(Full::new(Bytes::default()))
                    .unwrap();
                return Ok(response);
            }
        } else {
            let body = format!("No servers available, please try again"); //writes the body of html file
            let response = Response::builder()
                .status(500)
                .body(Full::new(Bytes::from(body)))
                .unwrap();
            return Ok(response);
        }
    }
}

async fn get_request<T>(
    req: Arc<Option<Request<hyper::body::Incoming>>>,
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) -> Option<Result<Response<Full<Bytes>>, Infallible>>
where
    T: LoadBalancer,
{
    let index = load_balancer.lock().unwrap().get_server();
    if index == None {
        return None;
    }
    let index = index.unwrap() as usize;

    let addr = config.lock().unwrap().servers[index].clone();

    println!("request forwarded to server {}", addr);

    let request = match &*req {
        Some(req) => format!("{}{}", addr, req.uri()),
        None => addr.to_string(),
    };

    let start = Instant::now();
    config.lock().unwrap().connections[index] += 1;

    let data = send_request(request).await;
    // println!("{:?}", data);

    let duration = start.elapsed();
    config.lock().unwrap().response_time[index] = duration;
    config.lock().unwrap().connections[index] -= 1;

    match data {
        Ok(data) => {
            config.lock().unwrap().alive[index] = true;
            return Some(Ok(Response::new(Full::new(data))));
        }
        Err(_) => {
            // Handle the error, such as logging it or returning an error response
            config.lock().unwrap().alive[index] = false;
            // eprintln!("Error occurred: {:?}", err);
            // Example error response, adjust as per your application logic
            // let response = Response::builder()
            // .status(500)
            // .body(Full::default())
            // .unwrap();
            // return Ok(response)
            return None;
        }
    };
}

async fn send_request(request: String) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    // Parse our URL...
    let url = request.parse::<hyper::Uri>()?;

    // Get the host and the port
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);

    let address = format!("{}:{}", host, port);

    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(address).await?;

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    // The authority of our URL will be the hostname of the httpbin remote
    let authority = url.authority().unwrap().clone();

    // Create an HTTP request with an empty body and a HOST header
    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    // Await the response...
    let mut res = sender.send_request(req).await?;

    println!("Response status: {}", res.status());

    let mut full_body = Vec::new();

    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            full_body.extend_from_slice(chunk);
        }
    }

    Ok(Bytes::from(full_body))
}

pub trait LoadBalancer {
    fn get_server(&self) -> Option<u32>;
}