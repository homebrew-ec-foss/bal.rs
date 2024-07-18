use std::convert::Infallible;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

async fn handel_request(_: Request<hyper::body::Incoming>, addr: Uri) -> Result<Response<Full<Bytes>>, Infallible> {
    let body = format!("responce from server {}", addr); //writes the body of html file
    let data = Full::new(Bytes::from(body));
    let response = Response::new(data);
    Ok(response)
}

#[tokio::main]
pub async fn start_server(addr: &Uri) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Server starting on http://{}", addr);

    let listener = TcpListener::bind(addr.to_string()).await?; //creates a TcpListener and bind it to 127.0.0.1:port

    //loops continuously accepting incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        
        let clint_addr = stream.peer_addr()?;
        let addr = addr.clone();
        println!("Accepted connection from {} (to server {})", clint_addr, addr);

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handel_request(req, addr.clone())))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

pub async fn start_servers(addrs: &Vec<Uri>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut tasks = vec![];

    for addr in addrs.clone() {
        let task = tokio::task::spawn_blocking(move || {
            match start_server(&addr) { //starts a server at port
                Ok(_) => println!("Server on port {} has terminated.", addr),
                Err(err) => eprintln!("Error running server on port {}: {}", addr, err),
            }
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}