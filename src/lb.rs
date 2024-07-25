use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::sleep;

use crate::{Algorithm, Config};
mod algos;
use algos::round_robin::RoundRobin;
use algos::static_lb::StaticLB;
use algos::weighted_round_robin::WeightedRoundRobin;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    let authority = uri
        .authority()
        .ok_or("URI does not have an authority part")?;
    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;
    let addr_str = format!("{}:{}", host, port);
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

fn servers_alive(status: &Vec<bool>) -> bool {
    status.iter().any(|&value| value)
}

#[tokio::main]
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Arc::new(Mutex::new(config));

    let static_lb = Arc::new(Mutex::new(StaticLB::new()));
    let len = config.lock().unwrap().servers.len();
    let health_check_interval = config.lock().unwrap().health_check_interval.clone();
    let config_clone = Arc::clone(&config);

    // Create a channel for queueing requests
    let (tx, rx) = mpsc::channel::<(Request<hyper::body::Incoming>, Sender<Response<Full<Bytes>>>)>(100);

    // Spawn a task for handling health checks
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

    // Spawn a task to process the request queue
    let config_clone = Arc::clone(&config);
    match algo {
        Algorithm::RoundRobin => {
            let load_balancer = Arc::new(Mutex::new(RoundRobin::new(Arc::clone(&config))));
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
        }
        Algorithm::WeightedRoundRobin => {
            let load_balancer = Arc::new(Mutex::new(WeightedRoundRobin::new(Arc::clone(&config))));
            tokio::spawn(process_queue(rx, Arc::clone(&config_clone), load_balancer));
        }
        Algorithm::LeastConnections => {}
        Algorithm::WeightedLeastConnections => {}
        Algorithm::LeastResponseTime => {}
        Algorithm::WeightedLeastResponseTime => {}
    }

    // Start listening for incoming connections
    listen(tx, config).await
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
            false,
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
    let index = load_balancer.lock().unwrap().get_server();
    if index.is_none() {
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
    let duration = start.elapsed();
    config.lock().unwrap().response_time[index] = duration;
    config.lock().unwrap().connections[index] -= 1;

    match data {
        Ok(data) => {
            config.lock().unwrap().alive[index] = true;
            Some(Ok(Response::new(Full::new(data))))
        }
        Err(_) => {
            config.lock().unwrap().alive[index] = false;
            None
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
    fn get_server(&self) -> Option<u32>;
}
