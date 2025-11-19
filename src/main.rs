use shell_words;
use std::{
    env, eprintln,
    io::{Write, stdin, stdout},
    process,
};

fn input() -> String {
    let mut str: String = String::new();
    stdin().read_line(&mut str).unwrap();

    str.trim().to_string()
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

        if receive_string.is_empty() {
            continue;
        }

        match receive_string.as_str() {
            "exit" | "quit" | "e" | "q" => break,
            command => {
                let args = match shell_words::split(command) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("Error parsing command: {}", e);
                        continue;
                    }
                };

                let mut prompt = process::Command::new(cmd);
                prompt.args(args);

                match prompt.spawn() {
                    Ok(mut subprocess) => {
                        let _ = subprocess.wait();
                    }
                    Err(e) => {
                        eprintln!("Failed to execute command: {}", e);
                    }
                }
            }
        }

        println!();
    }
}
