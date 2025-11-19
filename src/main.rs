use std::{env, eprintln, process};

fn main() {
    let args: Vec<String> = env::args().collect::<Vec<String>>();

    if args.len() <= 1 {
        eprintln!("Usage: with <COMMAND>");
        process::exit(1);
    }

    let cmd: &String = &args[1];
}
