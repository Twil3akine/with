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

    loop {
        print!("\x1b[36m{}\x1b[39m> ", &cmd);
        stdout().flush().unwrap();

        let receive_string: String = input();

        match receive_string.as_str() {
            "exit" | "quit" | "e" | "q" => break,
            command => {
                let args: Vec<&str> = command.split_whitespace().collect::<Vec<&str>>();
                let mut prompt = process::Command::new(cmd);
                prompt.args(args);
                prompt
                    .spawn()
                    .expect("Failed Ignission Command.")
                    .wait()
                    .expect("Happend Error Executing Command.");
            }
        }
        
        println!();
    }
}
