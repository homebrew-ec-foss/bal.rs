use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};    

use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Uri, Response};
use hyper_util::rt::TokioIo;
use std::str::FromStr;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{Algorithm, Config};
mod algos;
use algos::round_robin::RoundRobin;
use algos::weighted_round_robin::WeightedRoundRobin;
use algos::least_response_time::LeastResponseTime;
use algos::weighted_least_response_time::WeightedLeastResponseTime;
use algos::least_connections::LeastConnections;
use algos::weighted_least_connections::WeightedLeastConnections;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    let authority = uri
        .authority()
        .ok_or("URI does not have an authority part")?;

    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;

    let addr_str = format!("{}:{}", host, port);
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

#[tokio::main]
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Arc::new(Mutex::new(config));

    let (health_check_interval, len) = {
        let config_lock = config.lock().unwrap();
        (config_lock.health_check_interval, config_lock.servers.len())
    };
    let config_clone = Arc::clone(&config);

    let (tx, rx) = mpsc::channel::<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>(100);
    tokio::task::spawn(async move {
        loop {
            let mut tasks = Vec::new();

            for index in 0..len {
                let config_clone: Arc<Mutex<Config>> = Arc::clone(&config_clone);

                let task = tokio::task::spawn(async move {
                    let server = {
                        let mut config = config_clone.lock().unwrap();

                        if let Some(server) = config.servers.get_mut(index) {
                            server.connections += 1;
                        }

                        config.servers.get(index).cloned()
                    };
                    
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
                    let dead_server = {
                        let config = config_clone.lock().unwrap();

                        config.dead_servers.get(index).cloned()
                    };
                    
                    if let Some(dead_server) = dead_server {
                        let start = Instant::now();
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
                            }
                        }
                    }
                });
                tasks.push(task);
            }

            for task in tasks {
                drop(task.await);
            }

            println!("updated config | health checker");

            sleep(health_check_interval).await;
        }
    });

    let algo = {
        let config_lock = config.lock().unwrap();
        config_lock.algo.clone()
    };
    
    let config_clone = Arc::clone(&config);
    
    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
        Algorithm::LeastConnections => {
            let load_balancer = Arc::new(Mutex::new(LeastConnections::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
        Algorithm::WeightedLeastConnections => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastConnections::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
        Algorithm::LeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(LeastResponseTime::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
        Algorithm::WeightedLeastResponseTime => {
            let load_balancer = Arc::new(Mutex::new(WeightedLeastResponseTime::new()));
            let config_clone = Arc::clone(&config);
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
            drop(listen(tx.clone(), config_clone).await);
        }
    }

    Ok(())
}

async fn listen(
    tx: Sender<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>,
    config: Arc<Mutex<Config>>,
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

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let (response_tx, mut response_rx) = mpsc::channel(1);
                        tx_clone.try_send((req, response_tx)).unwrap();
                        async move { Ok::<_, Infallible>(response_rx.recv().await.unwrap()) }
                    }),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn process_queue<T>(
    mut rx: Receiver<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>,
    config: Arc<Mutex<Config>>,
    load_balancer: Arc<Mutex<T>>,
) where
    T: LoadBalancer + Send + 'static,
{
    while let Some((req, response_tx)) = rx.recv().await {
        let response = handle_request(
            Arc::new(Some(req)),
            Arc::clone(&config),
            Arc::clone(&load_balancer),
        )
        .await
        .unwrap();

        response_tx.send(response).await.unwrap();
    }
}

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
                    eprintln!("reruting request to a new server");
                }
            }
        } else {
            eprintln!("No servers available");
            let body = format!("No servers available, please try again");
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
    let server = {
        let mut config = config.lock().unwrap();

        let index_opt = load_balancer.lock().unwrap().get_index(Arc::new(&config));
        if index_opt == None {
            return None;
        }
        let index = index_opt.unwrap();

        config.servers[index].connections += 1;
        config.servers[index].clone()
    };

    let request = match &*req {
        Some(req) => format!("{}{}", server.addr.clone(), req.uri().to_string().trim_start_matches("/")),
        None => server.addr.to_string(),
    };

    println!("forwarded request to {:?}", request);

    let start = Instant::now();

    let data = send_request(request).await;

    let duration = start.elapsed();

    let mut config = config.lock().unwrap();

    let index = config.servers.iter().position(|c_server| c_server.addr == server.addr);
        
    if let Some(index) = index {
        config.servers[index].response_time = duration;
        config.servers[index].connections -= 1;
    }

    match data {
        Ok(data) => {
            return Some(Ok(Response::new(Full::new(data))));
        },
        Err(_) => {
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
    let url = request.parse::<hyper::Uri>()?;

    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);

    let address = format!("{}:{}", host, port);

    let stream = TcpStream::connect(address).await?;

    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

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
