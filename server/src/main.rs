mod server;
mod get_uris;

#[tokio::main]
async fn main() {
    let uris = get_uris::get_uris("server/servers.csv".to_string()); //gets all the ports from ports.csv

    match uris {
        Ok(uris) => {
            if let Err(err) = server::start_servers(&uris).await { //starts server on all the ports
                eprintln!("Error running servers: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Error retrieving ports: {}", err);
        }
    }
}