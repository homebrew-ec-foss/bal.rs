pub fn help() {
    println!("h or ? -> Displays this list of available commands");
    println!("q -> Quit program. Applicable only when program is run with no arguments");
    println!("start -> Starts the load balancer(Allegedly)");
    println!("stop -> Stops the load balancer(Allegedly)");
    println!("p <port_number> -> Changes to specified port. Takes one more argument as port number");
    println!("c -> Shows connected servers");
}

pub fn start() {
    println!("Yaay you started something. Have one bun samosa");
    
}

pub fn stop() {
    println!("Maybe you should stop sleeping all day like you stopped this");
}

pub fn ports(p: u32) {
    println!("You tried to do something with port {p} but idk what");
}

pub fn con_servers() -> u32 {
    0
}
