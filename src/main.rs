mod context;
mod executor;
mod parser;
mod with_helper;

use context::*;
use executor::execute_child_process;
use parser::*;
use rustyline::{
    Cmd, CompletionType, Config, Editor, KeyCode, Modifiers, Movement, Result, error::ReadlineError,
};
use std::{
    env, eprintln, format,
    option::Option::{None, Some},
    path::{Path, PathBuf},
    println, process,
    result::Result::Ok,
};
use with_helper::WithHelper;

fn print_help() {
    println!("With - Command Wrapper Tool");
    println!();
    println!("Usage:");
    println!("  <command> [args]  Execute command in the target context");
    println!("  cd <path>         Change current directory");
    println!("  ! <command>       Execute external command (e.g. !ls, !vim)");
    println!("  clear/cls         Clear the screen");
    println!("  help              Show this help message");
    println!("  pwd               Show current pass");
    println!("  history           Show command history");
    println!("  exit/quit (e/q)   Exit the application");
    println!();
    println!("Keyboard Shortcuts:");
    println!("  Ctrl + C          Cancel input / Interrupt process");
    println!("  Ctrl + D          Exit (EOF)");
    println!("  Tab               File completion");
}

// --- メインループ ---
/// REPL（対話型ループ）のメインロジック
fn run_repl(target_ctx: Option<&TargetContext>, base_path: &Path) -> Result<()> {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();

    let context_program = target_ctx.map(|ctx| ctx.program.clone());

    // エディタの初期化
    let mut rl = Editor::<WithHelper, rustyline::history::DefaultHistory>::with_config(config)?;
    rl.set_helper(Some(WithHelper {
        completer: rustyline::completion::FilenameCompleter::new(),
        context_program,
    }));

    // キーバインド設定: Escキーで入力行を全削除（Windowsライクな挙動）
    rl.bind_sequence(
        rustyline::KeyEvent(KeyCode::Esc, Modifiers::NONE),
        Cmd::Kill(Movement::WholeLine),
    );

    loop {
        let current_dir = env::current_dir().unwrap_or_default();
        let dir_name_opt = resolve_display_dir(&current_dir, base_path);

        let branch_opt = get_git_branch(&current_dir);

        // ディレクトリ情報とブランチ情報を結合する
        let context_info = match (dir_name_opt, branch_opt) {
            (Some(dir), Some(branch)) => Some(format!("{}: {}", dir, branch)),
            (Some(dir), None) => Some(dir),
            (None, Some(branch)) => Some(branch), // dir変化なしでもbranchがあれば出す場合
            (None, None) => None,
        };

        let prompt_cmd_str = if let Some(ctx) = target_ctx {
            if ctx.args.is_empty() {
                ctx.program.clone()
            } else {
                format!("{} {}", ctx.program, ctx.args.join(" "))
            }
        } else {
            String::new()
        };

        let prompt = match (target_ctx, context_info) {
            (Some(_cmd), Some(info)) => format!("({}) {}> ", info, prompt_cmd_str),
            (Some(_cmd), None) => format!("{}> ", prompt_cmd_str),
            (None, Some(info)) => format!("({}) > ", info),
            (None, None) => "> ".to_string(),
        };

        // ユーザーの入力を待機
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();

                if !line.is_empty() {
                    rl.add_history_entry(line)?;
                }

                let action = parse_cmd(line, target_ctx);

                match action {
                    CommandAction::Execute { program, args } => {
                        execute_child_process(&program, args);
                    }
                    CommandAction::ChangeDirectory(target) => {
                        if let Some(path) = target
                            && let Err(e) = env::set_current_dir(&path)
                        {
                            eprintln!("Failed to change directory: {}", e);
                        }
                    }
                    CommandAction::Clear(args) => {
                        let program = "clear";
                        execute_child_process(program, args);
                    }
                    CommandAction::Pwd(args) => {
                        let program = "pwd";
                        execute_child_process(program, args);
                    }
                    CommandAction::History => {
                        for (idx, history) in rl.history().iter().enumerate() {
                            println!("{: >3}: {}", idx + 1, history);
                        }
                    }
                    CommandAction::Help => {
                        print_help();
                    }
                    CommandAction::DoNothing => {}
                    CommandAction::Exit => break,
                    CommandAction::Error(msg) => eprintln!("Error: {}", msg),
                }
            }
            // Ctrl+C, Ctrl+D で終了した場合
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            // その他のエラー
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
        // 実行完了後に空行を入れて見やすくする
        println!();
    }
    Ok(())
}

// --- エントリーポイント ---
fn main() {
    // Rustylineの入力待ち中のCtrl+Cは、Rustyline側が別途ハンドリングしてくれます。
    ctrlc::set_handler(|| {}).expect("Error setting Ctrl-C handler");

    // コマンドライン引数を取得
    let args: Vec<String> = env::args().collect::<Vec<String>>();

    let target_ctx: Option<TargetContext> = if args.len() >= 2 {
        let joined_args = args[1..].join(" ");

        let split_args = shell_words::split(&joined_args).unwrap_or_default();

        if split_args.is_empty() {
            None
        } else {
            Some(TargetContext {
                program: split_args[0].clone(),
                args: split_args[1..].to_vec(),
            })
        }
    } else {
        None
    };

    let base_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if let Err(e) = run_repl(target_ctx.as_ref(), &base_path) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
