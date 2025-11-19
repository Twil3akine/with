use std::{
    env, eprintln,
    io::{Write, stdin, stdout},
    process,
};

fn input() -> String {
    let mut str: String = String::new();
    stdin().read_line(&mut str).unwrap();

    str.trim().parse().unwrap()
}

fn main() {
    let args: Vec<String> = env::args().collect::<Vec<String>>();

    if args.len() <= 1 {
        eprintln!("Usage: with <COMMAND>");
        process::exit(1);
    }

    let cmd: &String = &args[1];
}
