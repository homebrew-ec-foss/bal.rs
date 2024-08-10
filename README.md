# Bal.rs
```text
 ________  ________  ___           ________  ________          
|\   __  \|\   __  \|\  \         |\   __  \|\   ____\         
\ \  \|\ /\ \  \|\  \ \  \        \ \  \|\  \ \  \___|_        
 \ \   __  \ \   __  \ \  \        \ \   _  _\ \_____  \       
  \ \  \|\  \ \  \ \  \ \  \____  __\ \  \\  \\|____|\  \      
   \ \_______\ \__\ \__\ \_______\\__\ \__\\ _\ ____\_\  \     
    \|_______|\|__|\|__|\|_______\|__|\|__|\|__|\_________\    
                                               \|_________|   
L7 Load Balancer Implemented in Rust 🦀
```
> This project is part of [Tilde 3.0](https://github.com/homebrew-ec-foss/Tilde-3.0) HSP PESU-ECC's summer mentoring program.

## Mentees
- [Pushkar G R](https://github.com/pushkar-gr)
- [Pranav V Bhat](https://github.com/Prana-vvb)
- [Rohan Cyriac](https://github.com/rohancyriac029)
- [Raahithya J](https://github.com/Raahithyajayaram)

## Mentors
- [Adhesh Athrey](https://github.com/DedLad)
- [Anirudh Sudhir](https://github.com/anirudhsudhir)
- [Aditya Hegde](https://github.com/bwaklog)

## Getting started
To build the application locally, ensure you have the [Rust compiler and Cargo package manager installed](https://doc.rust-lang.org/book/ch01-01-installation.html). Once installed, clone the repository and build the application using Cargo:
```sh
git clone https://github.com/homebrew-ec-foss/bal.rs
cd bal.rs
cargo build
```
For a production-ready build, you can use:
```sh
cargo build --release
```

## How to Use
### The Application
After building, the main executable will be located in `/target/debug` or `/target/release` based on the build command used.
Navigate to the directory and type
```sh
Balrs help
```
in the terminal to get a list of available commands.

Alternatively, from the root directory of Bal.rs, you can use:
```sh
cargo run help
```
for the same result.

### The Config file
The `config.yaml` file allows you to customize the Load Balancer settings:

- <mark>**Load Balancer address**</mark>: The URL of the Load Balancer(Default: http://localhost:8000)
- <mark>**Algorithm**</mark>: The load balancing algorithm to be used by Balrs(Default: round robin)
- <mark>**Servers**</mark>: List of server URLs to which Balrs can send requests
- <mark>**Weights**</mark>: List of weights of respective servers, used by weighted load balancing algorithms
- <mark>**Max connections**</mark>: List of the maximum number of connections each server is allowed to handle
- <mark>**Timeout**</mark>: Maximum time allowed for a server to respond before it is marked as dead
- <mark>**Health check interval**</mark>: Time interval at which server health checks are performed
