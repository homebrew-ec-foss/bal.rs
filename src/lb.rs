use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::Response;
use tokio::net::TcpStream;
use hyper::{Request, body::Bytes};
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Empty, Full};
use tokio::net::TcpListener;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Uri;
use std::str::FromStr;
use std::time::Instant;

use crate::Config;
mod algos;
use algos::round_robin::RoundRobin;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> { // takes Uri and returns SocketAddr
    let authority = uri.authority().ok_or("URI does not have an authority part")?; // Ensure the URI has an authority part (host and port)
    
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
pub async fn start_lb(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // starts the load balancer
    let config = Arc::new(Mutex::new(config));

    let load_balancer = Arc::new(Mutex::new(RoundRobin::new(Arc::clone(&config))));

    let addr = uri_to_socket_addr(&config.lock().unwrap().load_balancer).unwrap();
    
    println!("Server is running on http://{}", addr);
    let listener = TcpListener::bind(addr).await?; // We create a TcpListener and bind it to load balancer address

    loop { // starting a loop to continuously accept incoming connections
        let (stream, _) = listener.accept().await?;
        let client_addr = stream.peer_addr()?;
        println!("Accepted connection from {} to load balancer {}", client_addr, addr);

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let config_clone = Arc::clone(&config);
        let load_balancer_clone = Arc::clone(&load_balancer);

        // !!!!!!!!!! idk smtg virtual thread thing should go down here
        tokio::task::spawn(async move { // spawns a tokio task to server multiple connections concurrently
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| {handle_request(Arc::new(req), Arc::clone(&config_clone), Arc::clone(&load_balancer_clone))})).await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn handle_request<T>(req: Arc<Request<hyper::body::Incoming>>, config: Arc<Mutex<Config>>, load_balancer: Arc<Mutex<T>>) -> Result<Response<Full<Bytes>>, Infallible>
where
    T: LoadBalancer,
{
    loop {
        if servers_alive(&config.lock().unwrap().alive) {
            match get_request(Arc::clone(&req), Arc::clone(&config), Arc::clone(&load_balancer)).await {
                Some(request) => {
                    return request;
                },
                None => {
                    println!("running again");
                }
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

async  fn get_request<T>(req: Arc<Request<hyper::body::Incoming>>, config: Arc<Mutex<Config>>, load_balancer: Arc<Mutex<T>>) -> Option<Result<Response<Full<Bytes>>, Infallible>>
where
    T: LoadBalancer,
{
    let index = load_balancer.lock().unwrap().get_server() as usize;
    
    let addr= config.lock().unwrap().servers[index].clone();

    println!("request forwarded to server {}", addr);
    
    let request = format!("{}{}", addr, req.uri());
    
    let start = Instant::now();
    config.lock().unwrap().connections[index] += 1;
    
    let data = send_request(request).await;
    // println!("{:?}", data);
        
    let duration = start.elapsed();
    config.lock().unwrap().response_time[index] = duration;
    config.lock().unwrap().connections[index] -= 1;
        
    match data {
        Ok(data) => {
            return Some(Ok(Response::new(Full::new(data))));
        }
        Err(_err) => {
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

async fn send_request(request:String) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
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
    fn get_server(&self) -> u32;
}