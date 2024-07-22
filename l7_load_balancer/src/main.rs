mod round_robin;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyper::Response;
use tokio::io::{self, AsyncWriteExt as _};
use tokio::net::TcpStream;
use hyper::{Request, body::Bytes};
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Empty, Full};
use tokio::net::TcpListener;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn handle_request(_: Request<hyper::body::Incoming>, ports_clone: Arc<Mutex<Vec<u16>>>) -> Result<Response<Full<Bytes>>, Infallible> {
    let port = match ports_clone.lock() {
        Ok(mut guard) => {
            round_robin::get_next_backend_port(&mut guard)
        },
        Err(poisoned) => {
            eprintln!("Mutex poisoned: {:?}", poisoned);
            // Handle the poisoned mutex state here if needed
            3000
        }
    };
    println!("request forwarded to port {}", port);
    let data = send_request(port).await;
    match data {
        Ok(data) => {
            Ok(Response::new(Full::new(data)))
        }
        Err(err) => {
            // Handle the error, such as logging it or returning an error response
            eprintln!("Error occurred: {:?}", err);
            // Example error response, adjust as per your application logic
            let response = Response::builder()
                .status(500)
                .body(Full::default())
                .unwrap();
            Ok(response)
        }
    }
}

async fn send_request(port: u16) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    // Parse our URL...
    let url = format!("http://127.0.0.1:{}", port).parse::<hyper::Uri>()?; //change 3000 after making round robin part

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ports: Arc<Mutex<Vec<u16>>> = Arc::new(Mutex::new(vec![3000, 3001, 3002, 3003, 3004]));
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    println!("Server is running on http://{}", addr);

    // We create a TcpListener and bind it to 127.0.0.1:8000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let addr = stream.peer_addr()?;
        println!("Accepted connection from {} (to port {})", addr, 8000);

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let ports_clone: Arc<Mutex<Vec<u16>>> = Arc::clone(&ports);
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(move |req| handle_request(req, Arc::clone(&ports_clone))))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }

}
