use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::time::{Duration, Instant};
use reqwest::Client;
use http_body_util::{Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
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
    let authority = uri.authority().ok_or("URI does not have an authority part")?;
    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;
    let addr_str = format!("{}:{}", host, port);
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

#[tokio::main]
pub async fn start_lb(lb: LoadBalancer) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, mut rx) = mpsc::channel::<(String, Instant)>(100000);

    let request_counter = Arc::new(RwLock::new(RequestCounter::new()));

    tokio::spawn(async move {
        while let Some((uri, timestamp)) = rx.recv().await {
            println!("Request to {} received at {:?}", uri, timestamp);
            // Additional processing can be done here
        }
    });

    let lb = Arc::new(Mutex::new(lb));

    let (health_check_interval, len, timeout_duration) = {
        let lb_lock = lb.lock().unwrap();
        (
            lb_lock.health_check_interval,
            lb_lock.servers.len(),
            lb_lock.timeout,
        )
    };
    let lb_clone = Arc::clone(&lb);

    tokio::task::spawn(async move {
        loop {
            let mut tasks = Vec::new();

            for index in 0..len {
                let lb_clone: Arc<Mutex<LoadBalancer>> = Arc::clone(&lb_clone);

                let task = tokio::task::spawn(async move {
                    let server = {
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
                    .await;
                    let duration = start.elapsed();

                    let mut lb = lb_clone.lock().unwrap();

                    lb.servers[index].response_time = duration;

                    if lb.servers[index].connections > 0 {
                        lb.servers[index].connections -= 1;
                    }

                    match response {
                        Ok(response) => {
                            if response.is_err()
                                || lb.servers[index].connections
                                    >= lb.servers[index].max_connections
                            {
                                lb.servers[index].connections = 0;
                                lb.servers[index].alive = false;
                            } else {
                                lb.servers[index].alive = true;
                            }
                        }
                        Err(_) => {
                            lb.servers[index].connections = 0;
                            lb.servers[index].alive = false;
                        }
                    }
                });
                tasks.push(task);
            }

            for task in tasks {
                drop(task.await);
            }

            let lb_clone2: Arc<Mutex<LoadBalancer>> = Arc::clone(&lb_clone);
            println!("Updated load balancer | Health checker");
            for i in lb_clone2
                .lock()
                .unwrap()
                .servers
                .iter()
                .filter(|server| server.alive && server.connections < server.max_connections)
            {
                println!(
                    "{} | active connections: {} | response time: {:?}",
                    i.addr, i.connections, i.response_time
                );
            }

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
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new()));
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
        Algorithm::LeastConnections => {
            let load_balancer = Arc::new(Mutex::new(LeastConnections::new()));
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
        Algorithm::WeightedLeastConnections => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastConnections::new()));
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
        Algorithm::LeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(LeastResponseTime::new()));
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
        Algorithm::WeightedLeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastResponseTime::new()));
            drop(listen(lb, load_balancer, tx, request_counter).await);
        }
    }

    Ok(())
}

async fn listen<T>(
    lb: Arc<Mutex<LoadBalancer>>,
    load_balancer: Arc<Mutex<T>>,
    tx: mpsc::Sender<(String, Instant)>,
    request_counter: Arc<RwLock<RequestCounter>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: Loadbalancer + Send + 'static,
{
    let addr = uri_to_socket_addr(&lb.lock().unwrap().load_balancer).unwrap();
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("Load balancer is running on http://{}", addr);
            listener
        }
        Err(err) => {
            eprintln!("{}", err);
            return Ok(());
        }
    };

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);

        let lb_clone = Arc::clone(&lb);
        let load_balancer_clone = Arc::clone(&load_balancer);
        let tx_clone = tx.clone();
        let request_counter_clone = Arc::clone(&request_counter);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        handle_request(
                            req,
                            Arc::clone(&lb_clone),
                            Arc::clone(&load_balancer_clone),
                            tx_clone.clone(),
                            Arc::clone(&request_counter_clone)
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
    req: Request<hyper::body::Incoming>,
    lb: Arc<Mutex<LoadBalancer>>,
    load_balancer: Arc<Mutex<T>>,
    tx: mpsc::Sender<(String, Instant)>,
    request_counter: Arc<RwLock<RequestCounter>>,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    T: Loadbalancer,
{
    let timeout_duration = lb.lock().unwrap().timeout;
    let uri_clone = req.uri().clone();
    let method_clone = req.method().clone();
    let client = Client::new();

    let mut request_counter = request_counter.write().await;
    request_counter.increment();
    let current_rate = request_counter.get_rate();
    drop(request_counter);

    if current_rate > 1000 {
        // Queue the request
        let _ = tx.send((uri_clone.to_string(), Instant::now())).await;
        return Ok(Response::new(Full::from(Bytes::from("Request queued due to high load"))));
    }

    loop {
        let (servers_alive, server_indices): (Vec<Server>, Vec<usize>) = {
            let lb = lb.lock().unwrap();
            lb.servers.iter().enumerate()
                .filter(|(_, server)| server.alive && server.connections < server.max_connections)
                .map(|(index, server)| (server.clone(), index))
                .unzip()
        };

        if servers_alive.is_empty() {
            println!("All servers are dead | No backend servers available");
            return Ok(Response::new(Full::from(Bytes::from("No backend servers available"))));
        }

        let servers_alive_refs: Vec<&Server> = servers_alive.iter().collect();

        let index_option = {
            let mut load_balancer = load_balancer.lock().unwrap();
            load_balancer.get_index(&servers_alive_refs)
        };

        if let Some(index) = index_option {
            let server_index = server_indices[index];
            let server = {
                let mut lb = lb.lock().unwrap();
                let server = &mut lb.servers[server_index];
                server.connections += 1;
                server.clone()
            };

            println!(
                "Sending request to {} | Active connections: {}",
                server.addr, server.connections
            );

            let url = format!("{}{}", server.addr, uri_clone.path());
            let request = client.request(method_clone.clone(), url.clone());

            match timeout(timeout_duration, request.send()).await {
                Ok(response) => match response {
                    Ok(resp) => {
                        let _ = tx.send((url.clone(), Instant::now())).await;
                        return Ok(Response::new(Full::from(Bytes::from(
                            resp.text().await.unwrap(),
                        ))));
                    }
                    Err(e) => {
                        eprintln!("Error executing request: {:?}", e);
                        // Mark server as dead or reduce its connection count
                        let mut lb = lb.lock().unwrap();
                        if let Some(server) = lb.servers.get_mut(server_index) {
                            if server.connections > 0 {
                                server.connections -= 1;
                            }
                            server.alive = false;
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Request timed out or failed: {:?}", e);
                    // Mark server as dead or reduce its connection count
                    let mut lb = lb.lock().unwrap();
                    if let Some(server) = lb.servers.get_mut(server_index) {
                        if server.connections > 0 {
                            server.connections -= 1;
                        }
                        server.alive = false;
                    }
                }
            }
        } else {
            // If no index was found, break the loop to avoid infinite loop
            break;
        }
    }

    // If all servers are exhausted and none could handle the request
    Ok(Response::new(Full::from(Bytes::from("No backend servers available"))))
}

struct RequestCounter {
    requests: u64,
    last_checked: Instant,
}

impl RequestCounter {
    fn new() -> Self {
        RequestCounter {
            requests: 0,
            last_checked: Instant::now(),
        }
    }

    // Ensure `get_rate` method takes a mutable reference
    fn get_rate(&mut self) -> u64 {
        let now = Instant::now();
        let duration = now.duration_since(self.last_checked);

        if duration.as_secs() > 1 {
            // Reset the counter and update the last_checked timestamp
            self.requests = 0;
            self.last_checked = now;
        }
        self.requests
    }

    // Method to increment the request count
    fn increment(&mut self) {
        self.requests += 1;
    }
}

pub trait Loadbalancer {
    fn get_index(&mut self, servers_alive: &[&Server]) -> Option<usize>;
}
