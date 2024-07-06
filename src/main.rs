use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    // Hardcoded ports for servers as of now
    let backends = vec![
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
    ];

    let backends = Arc::new(backends);//ref counting shared bw threads
    let counter = Arc::new(Mutex::new(0));

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();

    loop {
        let (client, _) = listener.accept().await.unwrap();
        let backends = Arc::clone(&backends);
        let counter = Arc::clone(&counter);

        tokio::spawn(async move {  //acquire lock on counter and backends and spawn threads for clients
            handle_client(client, backends, counter).await;
        });
    }
}

async fn handle_client(client: TcpStream, backends: Arc<Vec<String>>, counter: Arc<Mutex<usize>>) {
    //round robin simple ez 👍👍
    let backend_addr = {
        let mut counter = counter.lock().unwrap();
        let backend_addr = backends[*counter].clone();
        *counter = (*counter + 1) % backends.len();
        backend_addr
    };

    if let Ok(mut backend) = TcpStream::connect(backend_addr).await {
        let (mut client_reader, mut client_writer) = client.into_split();
        let (mut backend_reader, mut backend_writer) = backend.into_split();

        let client_to_backend = tokio::spawn(async move { //simple functions to copy stuff in both directions
            tokio::io::copy(&mut client_reader, &mut backend_writer).await.ok();
        });

        let backend_to_client = tokio::spawn(async move {
            tokio::io::copy(&mut backend_reader, &mut client_writer).await.ok();
        });

        let _ = tokio::try_join!(client_to_backend, backend_to_client); //wait for finish signal
    }
}
