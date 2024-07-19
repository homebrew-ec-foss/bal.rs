use std::{env, process, io};
use std::io::Write;
use Balrs::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        run(args);
    }
    else {
        println!(r#" ________  ___  ________  ________  ___  ___  ________      "#);
        println!(r#"|\   ____\|\  \|\   __  \|\   ____\|\  \|\  \|\   ____\     "#);
        println!(r#"\ \  \___|\ \  \ \  \|\  \ \  \___|\ \  \\\  \ \  \___|_    "#);
        println!(r#" \ \  \    \ \  \ \   _  _\ \  \    \ \  \\\  \ \_____  \   "#);
        println!(r#"  \ \  \____\ \  \ \  \\  \\ \  \____\ \  \\\  \|____|\  \  "#);
        println!(r#"   \ \_______\ \__\ \__\\ _\\ \_______\ \_______\____\_\  \ "#);
        println!(r#"    \|_______|\|__|\|__|\|__|\|_______|\|_______|\_________\"#);
        println!("Type h or ? for a list of available commands");

        let cli = whoami::username() + &String::from("@circus");

        loop {
            let mut arg = String::new();
            print!("{cli}:> ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut arg).unwrap();
            let mut args: Vec<String> = arg.trim().split_whitespace().map(String::from).collect();
            args.insert(0, "Blank".to_string());

            run(args);
        }
    }
}

fn run(args: Vec<String>) {
    match args[1].as_str() {
        "h" | "?" => help(),
        "start" => start(),
        "stop" => stop(),
        "p" => {
            let p: u32 = match args[2].trim().parse() {
                Ok(prt) => prt,
                Err(_) => {
                    println!("Error: Invalid argument passed as port number");
                    process::exit(0);
                }
            };
            ports(p);
        },
        "c" => {
            let s_count = con_servers();
            println!("Connected to {s_count} servers");
        },
        "q" => {
            println!("You gonna end me fr?");
            process::exit(0);
        },
        _ => println!("Unknown argument passed")
    }
}