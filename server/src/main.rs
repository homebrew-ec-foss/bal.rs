mod server;
mod get_ports;

#[tokio::main]
async fn main() {
    let ports = get_ports::get_ports("server/ports.csv".to_string()); //gets all the ports from ports.csv

    match ports {
        Ok(ports) => {
            if let Err(err) = server::start_servers(&ports).await { //starts server on all the ports
                eprintln!("Error running servers: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Error retrieving ports: {}", err);
        }
    }
}