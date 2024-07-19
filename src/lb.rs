use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server, Uri};
use hyper::http::StatusCode;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::str::FromStr;
use std::error::Error;

mod algos;

use algos::round_robin::RoundRobin;

use algos::weighted_round_robin::WeightedRoundRobin;

use crate::Config;

fn uri_to_socket_addr(uri: &Uri) -> Result<SocketAddr, &'static str> {
    // Ensure the URI has an authority part (host and port)
    let authority = uri.authority().ok_or("URI does not have an authority part")?;
    
    // Extract host and port
    let host = authority.host();
    let port = authority.port_u16().ok_or("URI does not have a port")?;
    
    // Combine host and port into a SocketAddr
    let addr_str = format!("{}:{}", host, port);
    SocketAddr::from_str(&addr_str).map_err(|_| "Failed to parse SocketAddr")
}

#[tokio::main]
pub async fn start_lb(config: &Config) -> Result<(), Box<dyn Error>> {
    let addr = uri_to_socket_addr(&config.load_balancer)?;

    let backend = &config.servers;

    let mut backends = Vec::new();

    for uri in backend {
        backends.push(format!("http://{}", uri.to_string()));
    }


    let mut backend = match config.algo {
        crate::Algorithm::round_robin => RoundRobin::new(backends),
        crate::Algorithm::weighted_round_robin => WeightedRoundRobin::new(backends), // need to change
        crate::Algorithm::least_connections => (),
        crate::Algorithm::weighted_least_connections => (),
        crate::Algorithm::least_response_time => (),
        crate::Algorithm::weighted_least_response_time => (),
    };

    start_server(addr, backend).await;

    Ok(())
}

async fn start_server<T>(addr: SocketAddr, load_balancer: Arc<T>)
where
    T: LoadBalancer + Send + Sync + 'static,
{
    let make_svc = make_service_fn(move |_| {
        let load_balancer = load_balancer.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let load_balancer = load_balancer.clone();
                async move {
                    match forward_request(req, load_balancer).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            eprintln!("Request failed: {}", e);
                            Ok::<_, Infallible>(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from("Internal Server Error"))
                                .unwrap())
                        }
                    }
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

async fn forward_request<T>(
    req: Request<Body>,
    load_balancer: Arc<T>,
) -> Result<Response<Body>, hyper::Error>
where
    T: LoadBalancer + Send + Sync + 'static,
{
    let backend = load_balancer.get_server();

    let client = Client::new();

    let uri_string = format!("{}{}", backend, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
    let uri: Uri = uri_string.parse().unwrap();

    let mut new_req_builder = Request::builder()
        .method(req.method())
        .uri(uri);

    for (key, value) in req.headers().iter() {
        new_req_builder = new_req_builder.header(key, value);
    }

    let new_req = new_req_builder
        .body(req.into_body())
        .unwrap();

    client.request(new_req).await
}

pub trait LoadBalancer {
    fn get_server(&self) -> String;
}