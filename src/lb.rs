use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use std::str::FromStr;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};

use crate::{Algorithm, LoadBalancer, Server};
mod algos;
use algos::least_connections::LeastConnections;
use algos::least_response_time::LeastResponseTime;
use algos::round_robin::RoundRobin;
use algos::weighted_least_connections::WeightedLeastConnections;
use algos::weighted_least_response_time::WeightedLeastResponseTime;
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

#[tokio::main]
pub async fn start_lb(lb: LoadBalancer) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // starts the load balancer
    let lb = Arc::new(Mutex::new(lb));

    let (health_check_interval, len, timeout_duration) = {
        // gets health check interval and number of servers
        let lb_lock = lb.lock().unwrap();
        (
            lb_lock.health_check_interval,
            lb_lock.servers.len(),
            lb_lock.timeout,
        )
    };
    let lb_clone = Arc::clone(&lb); //creates a clone for health checker

    tokio::task::spawn(async move {
        loop {
            let mut tasks = Vec::new();

            for index in 0..len {
                let lb_clone: Arc<Mutex<LoadBalancer>> = Arc::clone(&lb_clone);

                let task = tokio::task::spawn(async move {
                    let server = {
                        // gets a local copy of server
                        let mut lb = lb_clone.lock().unwrap();

                        if let Some(server) = lb.servers.get_mut(index) {
                            server.connections += 1;
                        }

                        lb.servers[index].clone()
                    };

                    let start = Instant::now();
                    let response = timeout(
                        timeout_duration,
                        reqwest::get(server.addr.clone().to_string()),
                    )
                    .await; // sends a request to server
                    let duration = start.elapsed(); // get's the response time

                    let mut lb = lb_clone.lock().unwrap();
                    
                    lb.servers[index].response_time = duration;

                    lb.servers[index].connections -= 1;

                    match response {
                        Ok(response) => {
                            if response.is_err() || lb.servers[index].connections >= lb.servers[index].max_connections {
                                lb.servers[index].connections = 0;
                                lb.servers[index].alive = false;
                            } else {
                                lb.servers[index].alive = true;
                            }
                        },
                        Err(_) => {
                            lb.servers[index].connections = 0;
                            lb.servers[index].alive = false;
                        }
                    }
                });
                tasks.push(task);
            }

            for task in tasks {
                drop(task.await); // waits for all the servers to get updated
            }

            println!("updated lb | health checker");

            sleep(health_check_interval).await;
        }
    });

    let algo = {
        let lb_lock = lb.lock().unwrap();
        lb_lock.algo.clone()
    };

    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new()));
            drop(listen(lb, load_balancer).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new()));
            drop(listen(lb, load_balancer).await);
        }
        Algorithm::LeastConnections => {
            let load_balancer = Arc::new(Mutex::new(LeastConnections::new()));
            drop(listen(lb, load_balancer).await);
        }
        Algorithm::WeightedLeastConnections => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastConnections::new()));
            drop(listen(lb, load_balancer).await);
        }
        Algorithm::LeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(LeastResponseTime::new()));
            drop(listen(lb, load_balancer).await);
        }
        Algorithm::WeightedLeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastResponseTime::new()));
            drop(listen(lb, load_balancer).await);
        }
    }

    Ok(())
}

async fn listen<T>(
    lb: Arc<Mutex<LoadBalancer>>,
    load_balancer: Arc<Mutex<T>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: Loadbalancer + Send + 'static,
{
    let addr = uri_to_socket_addr(&lb.lock().unwrap().load_balancer).unwrap();
    let listener = match TcpListener::bind(addr).await {
        // We create a TcpListener and bind it to load balancer address
        Ok(listener) => {
            println!("load balancer is running on http://{}", addr);
            listener
        }
        Err(err) => {
            eprintln!("{}", err);
            return Ok(());
        }
    };

    loop {
        // starting a loop to continuously accept incoming connections
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let lb_clone = Arc::clone(&lb);
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
                            Arc::clone(&lb_clone),
                            Arc::clone(&load_balancer_clone),
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
    lb: Arc<Mutex<LoadBalancer>>,
    load_balancer: Arc<Mutex<T>>,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    T: Loadbalancer,
{
    loop {
        let len = lb.lock().unwrap().servers.iter().filter(|server| server.alive && server.connections < server.max_connections).collect::<Vec<&Server>>().len();
        if len > 0 {
            match get_request(
                Arc::clone(&req),
                Arc::clone(&lb),
                Arc::clone(&load_balancer),
            )
            .await
            {
                Some(request) => {
                    return request;
                }
                None => {
                    eprintln!("rerouting request to a new server");
                }
            }
        } else {
            eprintln!("No servers available");
            let body = "No servers available, please try again".to_string(); // writes the body of html file
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
    lb: Arc<Mutex<LoadBalancer>>,
    load_balancer: Arc<Mutex<T>>,
) -> Option<Result<Response<Full<Bytes>>, Infallible>>
where
    T: Loadbalancer,
{
    let (server, timeout_duration, index) = {
        // updates server details and gets a local copy of server
        let mut lb = lb.lock().unwrap();

        let servers_alive = lb.servers.iter().filter(|server| server.alive && server.connections < server.max_connections).collect();
        let index_opt = load_balancer.lock().unwrap().get_index(&servers_alive);
        index_opt?;

        let index = index_opt.unwrap();
        let server = servers_alive[index].clone();

        let index_opt = lb.servers.iter().position(|c_server| c_server.addr == server.addr); // get's the index of dead_server
        index_opt?;

        let index = index_opt.unwrap();

        lb.servers[index].connections += 1;
        (lb.servers[index].clone(), lb.timeout, index)
    };

    let request = match &*req {
        Some(req) => format!(
            "{}{}",
            server.addr.clone(),
            req.uri().to_string().trim_start_matches("/")
        ),
        None => server.addr.to_string(),
    }; // updates the address

    println!("forwarded request to {:?}", request);

    let start = Instant::now();

    let data = timeout(timeout_duration, send_request(request)).await; // sends request to the address
                                                                       // println!("{:?}", data);

    let duration = start.elapsed(); // gets response time

    let mut lb = lb.lock().unwrap();
    
    lb.servers[index].response_time = duration;

    lb.servers[index].connections -= 1;

    match data {
        Ok(data) => {
            match data {
                Ok(data) => Some(Ok(Response::new(Full::new(data)))),
                Err(_) => {
                    // sends server to `dead_servers` list if server is dead
                    lb.servers[index].connections = 0;
                    lb.servers[index].alive = false;
                    None
                }
            }
        }
        Err(_) => {
            // sends server to `dead_servers` list if server is dead
            if data.is_err() {
                lb.servers[index].connections = 0;
                lb.servers[index].alive = false;
            }
            None
        }
    }
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

pub trait Loadbalancer {
    fn get_index(&mut self, lb: &Vec<&Server>) -> Option<usize>;
}
