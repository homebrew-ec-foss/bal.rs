use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    // Hardcoded ports for servers as of now
    let backends = vec![
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];

    let backends = Arc::new(backends); //ref counting shared bw threads
    let counter = Arc::new(Mutex::new(0));

    // Start backend servers
    for backend in backends.iter() {
        let addr = backend.clone();
        tokio::spawn(async move {
            start_backend(addr).await;
        });
    }

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("client on localhost:8000");

    loop {
        let (client, _) = listener.accept().await.unwrap();
        println!("Connection established"); // Connection works

        let backends = Arc::clone(&backends);
        let counter = Arc::clone(&counter);

        tokio::spawn(async move { // Acquire lock on counter and backends and spawn threads for clients
            handle_client(client, backends, counter).await;
        });
    }
}

async fn handle_client(mut client: TcpStream, backends: Arc<Vec<String>>, counter: Arc<Mutex<usize>>) {
    // Round robin eziest 👍👍
    let backend_addr = {
        let mut counter = counter.lock().unwrap();
        let backend_addr = backends[*counter].clone();
        *counter = (*counter + 1) % backends.len();
        backend_addr
    };

    println!("Forwarding to: {}", backend_addr);

    if let Ok(mut backend) = TcpStream::connect(&backend_addr).await {
        let (mut client_reader, mut client_writer) = tokio::io::split(client);
        let (mut backend_reader, mut backend_writer) = tokio::io::split(backend);

        let client_to_backend = tokio::spawn(async move { // Copy data from client to backend
            let mut buffer = [0; 1024];
            while let Ok(n) = client_reader.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                backend_writer.write_all(&buffer[..n]).await.ok();
            }
        });

        let backend_to_client = tokio::spawn(async move {
            let mut buffer = [0; 1024];
            while let Ok(n) = backend_reader.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                client_writer.write_all(&buffer[..n]).await.ok();
            }
        });

        let _ = tokio::try_join!(client_to_backend, backend_to_client); // Wait for finish signal
    }
}

async fn start_backend(addr: String) {
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Backend server listening on {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let addr_clone = addr.clone(); // clone to have two independent addr values 
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            let n = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);
            println!("Received request on {}", addr_clone); // Use the cloned addr value

            if let Some(message) = extract_message(&request) {
                println!("Received on {}: {}", addr_clone, message);
            }

            
            let response = format!(
                //200 OK
                "HTTP/1.1 200 OK\r\n\r\n{}",
                extract_message(&request).unwrap_or_else(|| "No message found".to_string())
            );

            socket.write_all(response.as_bytes()).await.unwrap();
        });
    }
}

fn extract_message(request: &str) -> Option<String> {
    if request.contains("POST") {
        let body_start = request.find("\r\n\r\n").unwrap_or(request.len()) + 4; //body start
        let body = &request[body_start..];
        let message_prefix = "message=";
        body.split('&')
            .find(|pair| pair.starts_with(message_prefix)) //wish there was strtok in rust fr
            .map(|pair| pair[message_prefix.len()..].to_string())
    } else {
        None
    }
}
//error handling. cba to do rn