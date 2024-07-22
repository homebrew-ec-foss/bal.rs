use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

async fn handel_request(_: Request<hyper::body::Incoming>, port: u16) -> Result<Response<Full<Bytes>>, Infallible> {
    let body = format!("responce from port {}", port); //writes the body of html file
    let data = Full::new(Bytes::from(body));
    let response = Response::new(data);
    Ok(response)
}

#[tokio::main]
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server starting on http://{}", addr);

    let listener = TcpListener::bind(addr).await?; //creates a TcpListener and bind it to 127.0.0.1:port

    //loops continuously accepting incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        
        let addr = stream.peer_addr()?;
        println!("Accepted connection from {} (to port {})", addr, port);

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handel_request(req, port)))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

pub async fn start_servers(ports: &[u16]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tasks = vec![];

    for &port in ports {
        let task = tokio::task::spawn_blocking(move || {
            match start_server(port) { //starts a server at port
                Ok(_) => println!("Server on port {} has terminated.", port),
                Err(err) => eprintln!("Error running server on port {}: {}", port, err),
            }
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}