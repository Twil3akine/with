use rustyline::{
    Cmd, Completer, Editor, Helper, Hinter, KeyCode, Modifiers, Movement, Result, Validator,
    error::ReadlineError, highlight::Highlighter,
};
use shell_words;
use std::{borrow::Cow, env, eprintln, process};

#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if prompt.contains(">") {
            let styled = prompt.replace(">", "\x1b[39m>");
            Cow::Owned(format!("\x1b[36m{}", styled))
        } else {
            Cow::Borrowed(prompt)
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        eprintln!("Usage: with <COMMAND>");
        process::exit(1);
    }

    let cmd: &String = &args[1];
    let mut rl = Editor::<MyHelper, rustyline::history::DefaultHistory>::new()
        .expect("Failed Initialing editor.");
    rl.set_helper(Some(MyHelper {}));
    rl.bind_sequence(
        rustyline::KeyEvent(KeyCode::Esc, Modifiers::NONE),
        Cmd::Kill(Movement::WholeLine),
    );

    loop {
        let readline = rl.readline(&format!("{}> ", cmd));

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);
                match line {
                    "e" | "q" | "exit" | "quit" => {
                        return Ok(());
                    }

                    _ => {}
                }

                let args = match shell_words::split(line) {
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
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                return Ok(());
            }
            Err(e) => {
                println!("Error: {:?}", e);
                continue;
            }
        }

        println!();
    }
}
