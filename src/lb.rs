use std::convert::Infallible;
use std::net::SocketAddr;
<<<<<<< HEAD
use std::sync::{Arc, Mutex, MutexGuard};    

=======
use std::sync::{Arc, Mutex, MutexGuard};
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Uri, Response};
use hyper_util::rt::TokioIo;
use std::str::FromStr;
<<<<<<< HEAD
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, sleep};
=======
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::RwLock;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68

use crate::{Algorithm, Config};
mod algos;
use algos::round_robin::RoundRobin;
use algos::weighted_round_robin::WeightedRoundRobin;
use algos::least_response_time::LeastResponseTime;
use algos::weighted_least_response_time::WeightedLeastResponseTime;
use algos::least_connections::LeastConnections;
use algos::weighted_least_connections::WeightedLeastConnections;

<<<<<<< HEAD
fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> { // takes Uri and returns SocketAddr
    let authority = uri
        .authority()
        .ok_or("URI does not have an authority part")?; // Ensure the URI has an authority part (host and port)

    let host = authority.host(); // Extract host and port
    let port = authority.port_u16().ok_or("URI does not have a port")?;

    let addr_str = format!("{}:{}", host, port); // Combine host and port into a SocketAddr
=======
const REQUESTS_PER_SECOND_THRESHOLD: usize = 8000;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    let authority = uri
        .authority()
        .ok_or("URI does not have an authority part")?;

    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;

    let addr_str = format!("{}:{}", host, port);
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

#[tokio::main]
<<<<<<< HEAD
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // starts the load balancer
    let config = Arc::new(Mutex::new(config));

    let (health_check_interval, len, timeout_duration) = { // gets health check interval and number of servers
        let config_lock = config.lock().unwrap();
        (config_lock.health_check_interval, config_lock.servers.len(), config_lock.timeout)
    };
    let config_clone = Arc::clone(&config); //creates a clone for health checker

=======
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Arc::new(Mutex::new(config));
    let request_counter = Arc::new(RwLock::new(RequestCounter::new()));

    let (health_check_interval, len) = {
        let config_lock = config.lock().unwrap();
        (config_lock.health_check_interval, config_lock.servers.len())
    };
    let config_clone = Arc::clone(&config);

    let (tx, rx) = mpsc::channel::<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>(100000);    
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
    tokio::task::spawn(async move {
        loop {
            let mut tasks = Vec::new();

            for index in 0..len {
                let config_clone: Arc<Mutex<Config>> = Arc::clone(&config_clone);

                let task = tokio::task::spawn(async move {
<<<<<<< HEAD
                    let server = { // gets a local copy of server
                        let mut config = config_clone.lock().unwrap();
    
=======
                    let server = {
                        let mut config = config_clone.lock().unwrap();

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                        if let Some(server) = config.servers.get_mut(index) {
                            server.connections += 1;
                        }

                        config.servers.get(index).cloned()
                    };
                    
<<<<<<< HEAD
                    if let Some(server) = server { // checks if server exists
                        let start = Instant::now();
                        let response = timeout(timeout_duration, reqwest::get(server.addr.clone().to_string())).await; // sends a request to server
                        let duration = start.elapsed(); // get's the response time
    
                        let mut config = config_clone.lock().unwrap();
    
                        let index = config.servers.iter().position(|c_server| c_server.addr == server.addr); // get's the index of server
                        
                        if let Some(index) = index { // updates server data
                            config.servers[index].response_time = duration;
                            
                            config.servers[index].connections -= 1;
                        }
    
                        if let Err(_) = response { // sends server to dead server's list if server is dead
=======
                    if let Some(server) = server {
                        let start = Instant::now();
                        let response = reqwest::get(server.addr.clone().to_string()).await;
                        let duration = start.elapsed();

                        let mut config = config_clone.lock().unwrap();

                        let index = config.servers.iter().position(|c_server| c_server.addr == server.addr);
                        
                        if let Some(index) = index {
                            config.servers[index].response_time = duration;
                            config.servers[index].connections -= 1;
                        }

                        if let Err(_) = response {
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                            if let Some(index) = index {
                                let dead_server = config.servers.remove(index);
                                config.dead_servers.push(dead_server);
                            }
                        }
                    }
                });
                tasks.push(task);
            }

            for index in 0..len {
                let config_clone: Arc<Mutex<Config>> = Arc::clone(&config_clone);
                
                let task = tokio::task::spawn(async move {
<<<<<<< HEAD
                    let dead_server = { // gets a local copy of daed servers
                        let config = config_clone.lock().unwrap();
    
=======
                    let dead_server = {
                        let config = config_clone.lock().unwrap();

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                        config.dead_servers.get(index).cloned()
                    };
                    
                    if let Some(dead_server) = dead_server {
                        let start = Instant::now();
<<<<<<< HEAD
                        let response = timeout(timeout_duration, reqwest::get(dead_server.addr.clone().to_string())).await; // sends a request to server
                        let duration = start.elapsed(); // get's the response time
    
                        let mut config = config_clone.lock().unwrap();
    
                        let index = config.dead_servers.iter().position(|c_dead_server| c_dead_server.addr == dead_server.addr); // get's the index of dead_server
    
                        if let Ok(_) = response {
                            if let Some(index) = index {
                                let mut dead_server = config.dead_servers.remove(index);
                                dead_server.connections = 0; // resets number of connections
                                dead_server.response_time = duration; // updates response time
                                config.servers.push(dead_server); // sends dead server to `servers` vector
=======
                        let response = reqwest::get(dead_server.addr.clone().to_string()).await;
                        let duration = start.elapsed();

                        let mut config = config_clone.lock().unwrap();

                        let index = config.dead_servers.iter().position(|c_dead_server| c_dead_server.addr == dead_server.addr);

                        if let Ok(_) = response {
                            if let Some(index) = index {
                                let mut dead_server = config.dead_servers.remove(index);
                                dead_server.connections = 0;
                                dead_server.response_time = duration;
                                config.servers.push(dead_server);
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                            }
                        }
                    }
                });
                tasks.push(task);
            }

            for task in tasks {
<<<<<<< HEAD
                drop(task.await); // waits for all the servers to get updated
=======
                drop(task.await);
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
            }

            println!("updated config | health checker");

            sleep(health_check_interval).await;
        }
    });

    let algo = {
        let config_lock = config.lock().unwrap();
        config_lock.algo.clone()
    };
    
<<<<<<< HEAD
    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new()));
            drop(listen(config, load_balancer).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new()));
            drop(listen(config, load_balancer).await);
        }
        Algorithm::LeastConnections => {
            let load_balancer = Arc::new(Mutex::new(LeastConnections::new()));
            drop(listen(config, load_balancer).await);
        }
        Algorithm::WeightedLeastConnections => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastConnections::new()));
            drop(listen(config, load_balancer).await);
        }
        Algorithm::LeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(LeastResponseTime::new()));
            drop(listen(config, load_balancer).await);
        }
        Algorithm::WeightedLeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastResponseTime::new()));
            drop(listen(config, load_balancer).await);
=======
    let config_clone = Arc::clone(&config);
    
    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
        }
        Algorithm::LeastConnections => {
            let load_balancer = Arc::new(Mutex::new(LeastConnections::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
        }
        Algorithm::WeightedLeastConnections => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastConnections::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
        }
        Algorithm::LeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(LeastResponseTime::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
        }
        Algorithm::WeightedLeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastResponseTime::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone, Arc::clone(&request_counter)).await);
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
        }
    }

    Ok(())
}

<<<<<<< HEAD
async fn listen<T>(
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: LoadBalancer + Send + 'static,
{
    let addr = uri_to_socket_addr(&config.lock().unwrap().load_balancer).unwrap();
    let listener = match TcpListener::bind(addr).await { // We create a TcpListener and bind it to load balancer address
        Ok(listener) => {
            println!("load balancer is running on http://{}", addr);
            listener
        },
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

        let config_clone = Arc::clone(&config);
        let load_balancer_clone = Arc::clone(&load_balancer);

        // !!!!!!!!!! idk smtg virtual thread thing should go down here
        tokio::task::spawn(async move { // spawns a tokio task to server multiple connections concurrently
=======
async fn listen(
    tx: Sender<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>,
    config: Arc<Mutex<Config>>,
    request_counter: Arc<RwLock<RequestCounter>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = uri_to_socket_addr(&config.lock().unwrap().load_balancer).unwrap();
    println!("Server is running on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let client_addr = stream.peer_addr()?;
        println!(
            "Accepted connection from {} to load balancer {}",
            client_addr, addr
        );

        let io = TokioIo::new(stream);
        let tx_clone = tx.clone();
        let request_counter_clone = Arc::clone(&request_counter);

        tokio::task::spawn(async move {
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
<<<<<<< HEAD
                        handle_request(
                            Arc::new(Some(req)),
                            Arc::clone(&config_clone),
                            Arc::clone(&load_balancer_clone),   
                        )
=======
                        let (response_tx, mut response_rx) = mpsc::channel(1);
                        let request_counter_clone = Arc::clone(&request_counter_clone);
                        let tx_clone = tx_clone.clone();

                        async move {
                            let mut counter = request_counter_clone.write().await;
                            counter.add_request();

                            if counter.request_rate() > REQUESTS_PER_SECOND_THRESHOLD {
				let mut file = OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("request_rate_log.txt")
                                    .unwrap();
                                writeln!(file, "Request rate exceeded 8000 requests per second. Adding to the queue.")
                                    .unwrap();
                                if tx_clone.try_send((req, response_tx)).is_err() {
                                    eprintln!("Failed to send request to processing queue");
                                }
                            } else {
                                if response_tx
                                    .send(Response::builder()
                                        .status(200)
                                        .body(Full::new(Bytes::from("Request processed immediately")))
                                        .unwrap())
                                    .await
                                    .is_err()
                                {
                                    eprintln!("Failed to send immediate response");
                                }
                            }

                            match response_rx.recv().await {
                                Some(response) => Ok::<_, Infallible>(response),
                                None => {
                                    eprintln!("Failed to receive response");
                                    Ok::<_, Infallible>(Response::builder()
                                        .status(500)
                                        .body(Full::new(Bytes::from("Internal Server Error")))
                                        .unwrap())
                                }
                            }
                        }
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

<<<<<<< HEAD
=======
async fn process_queue<T>(
    mut rx: Receiver<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>,
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) where
    T: LoadBalancer + Send + 'static,
{
    while let Some((req, response_tx)) = rx.recv().await {
        match handle_request(
            Arc::new(Some(req)),
            Arc::clone(&config),
            Arc::clone(&load_balancer),
        )
        .await
        {
            Ok(response) => {
                if response_tx.send(response).await.is_err() {
                    eprintln!("Failed to send response");
                }
            }
            Err(_) => {
                eprintln!("Failed to handle request");
                let response = Response::builder()
                    .status(500)
                    .body(Full::new(Bytes::from("Internal Server Error")))
                    .unwrap();
                if response_tx.send(response).await.is_err() {
                    eprintln!("Failed to send error response");
                }
            }
        }
    }
}

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
async fn handle_request<T>(
    req: Arc<Option<Request<hyper::body::Incoming>>>,
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    T: LoadBalancer,
{
    loop {
        let len = config.lock().unwrap().servers.len();
        if len > 0 {
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
<<<<<<< HEAD
                    eprintln!("rerouting request to a new server");
=======
                    eprintln!("Rerouting request to a new server");
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
                }
            }
        } else {
            eprintln!("No servers available");
<<<<<<< HEAD
            let body = format!("No servers available, please try again"); // writes the body of html file
=======
            let body = format!("No servers available, please try again");
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
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
<<<<<<< HEAD
    let (server, timeout_duration) = { // updates server details and gets a local copy of server
=======
    let server = {
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
        let mut config = config.lock().unwrap();

        let index_opt = load_balancer.lock().unwrap().get_index(Arc::new(&config));
        if index_opt == None {
            return None;
        }
        let index = index_opt.unwrap();

        config.servers[index].connections += 1;
<<<<<<< HEAD
        (config.servers[index].clone(), config.timeout)
    };

    
    let request = match &*req {
        Some(req) => format!("{}{}", server.addr.clone(), req.uri().to_string().trim_start_matches("/")),
        None => server.addr.to_string(),
    }; // updates the address
=======
        config.servers[index].clone()
    };

    let request = match &*req {
        Some(req) => format!("{}{}", server.addr.clone(), req.uri().to_string().trim_start_matches("/")),
        None => server.addr.to_string(),
    };
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68

    println!("forwarded request to {:?}", request);

    let start = Instant::now();

<<<<<<< HEAD
    let data = timeout(timeout_duration, send_request(request)).await; // sends request to the address
    // println!("{:?}", data);

    let duration = start.elapsed(); // gets response time

    let mut config = config.lock().unwrap();

    let index = config.servers.iter().position(|c_server| c_server.addr == server.addr); // gets index of server
        
    if let Some(index) = index { // updates server details
        config.servers[index].response_time = duration;
        
=======
    let data = send_request(request).await;

    let duration = start.elapsed();

    let mut config = config.lock().unwrap();

    let index = config.servers.iter().position(|c_server| c_server.addr == server.addr);
        
    if let Some(index) = index {
        config.servers[index].response_time = duration;
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
        config.servers[index].connections -= 1;
    }

    match data {
        Ok(data) => {
<<<<<<< HEAD
            match data {
                Ok(data) => return Some(Ok(Response::new(Full::new(data)))),
                Err(_) => { // sends server to `dead_servers` list if server is dead
                    if let Err(_) = data {
                        if let Some(index) = index {
                            let dead_server = config.servers.remove(index);
                            config.dead_servers.push(dead_server);
                        }
                    }
                    return None;
                }
            };
        },
        Err(_) => { // sends server to `dead_servers` list if server is dead
=======
            return Some(Ok(Response::new(Full::new(data))));
        },
        Err(_) => {
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
            if let Err(_) = data {
                if let Some(index) = index {
                    let dead_server = config.servers.remove(index);
                    config.dead_servers.push(dead_server);
                }
            }
            return None;
        }
    }
}

async fn send_request(request: String) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
<<<<<<< HEAD
    // Parse our URL...
    let url = request.parse::<hyper::Uri>()?;

    // Get the host and the port
=======
    let url = request.parse::<hyper::Uri>()?;

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);

    let address = format!("{}:{}", host, port);

<<<<<<< HEAD
    // Open a TCP connection to the remote host
    let stream = TcpStream::connect(address).await?;

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Create the Hyper client
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn a task to poll the connection, driving the HTTP state
=======
    let stream = TcpStream::connect(address).await?;

    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

<<<<<<< HEAD
    // The authority of our URL will be the hostname of the httpbin remote
    let authority = url.authority().unwrap().clone();

    // Create an HTTP request with an empty body and a HOST header
=======
    let authority = url.authority().unwrap().clone();

>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

<<<<<<< HEAD
    // Await the response...
=======
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
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
    fn get_index(&mut self, config: Arc<&MutexGuard<Config>>) -> Option<usize>;
}
<<<<<<< HEAD
=======

struct RequestCounter {
    request_times: VecDeque<Instant>,
}

impl RequestCounter {
    pub fn new() -> Self {
        Self {
            request_times: VecDeque::with_capacity(REQUESTS_PER_SECOND_THRESHOLD),
        }
    }

    pub fn add_request(&mut self) {
        let now = Instant::now();
        self.request_times.push_back(now);
        self.clean_up(now);
    }

    pub fn request_rate(&self) -> usize {
        self.request_times.len()
    }

    fn clean_up(&mut self, now: Instant) {
        while let Some(&time) = self.request_times.front() {
            if now.duration_since(time) > Duration::from_secs(1) {
                self.request_times.pop_front();
            } else {
                break;
            }
        }
    }
}
>>>>>>> dfabea4d20c7f5d4b6780b2a0be5fdb349c3da68
