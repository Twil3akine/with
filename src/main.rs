mod context;
mod parser;
mod with_helper;

use context::*;
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
    println!("  exit/quit (e/q)   Exit the application");
    println!();
    println!("Keyboard Shortcuts:");
    println!("  Ctrl + C          Cancel input / Interrupt process");
    println!("  Ctrl + D          Exit (EOF)");
    println!("  Tab               File completion");
}

// --- コマンド実行処理 ---
/// 指定されたプログラムを子プロセスとして実行する関数
/// 失敗しても親プロセス（このREPL）はクラッシュさせない
fn execute_child_process(program: &str, args: Vec<String>) {
    let mut command = process::Command::new(program);
    command.args(args);

    // spawn() でプロセスを開始
    match command.spawn() {
        Ok(mut child) => {
            // wait() で子プロセスの終了を待機する（これがないと入力待ちとかぶる）
            if let Err(e) = child.wait() {
                eprintln!("Error waiting for process: {}", e);
            }
        }
        Err(e) => {
            // コマンドが見つからない、実行権限がないなどのエラー
            eprintln!("Failed to execute command '{}': {}", program, e);
        }
    }
}

// --- メインループ ---
/// REPL（対話型ループ）のメインロジック
fn run_repl(target_cmd: Option<&str>, base_path: &Path) -> Result<()> {
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();

    // エディタの初期化
    let mut rl = Editor::<WithHelper, rustyline::history::DefaultHistory>::with_config(config)?;
    rl.set_helper(Some(WithHelper {
        completer: rustyline::completion::FilenameCompleter::new(),
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

        let prompt = match (target_cmd, context_info) {
            (Some(cmd), Some(info)) => format!("({}) {}> ", info, cmd),
            (Some(cmd), None) => format!("{}> ", cmd),
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

                let action = parse_cmd(line, target_cmd);

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
                        // 既存の関数を使って実行 (これで clear -x なども動くようになります)
                        execute_child_process(program, args);
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

    // 引数が足りない場合（自分自身 + コマンド名 の2つ必要）
    if args.len() > 2 {
        eprintln!("Usage: with none | <COMMAND>");
        process::exit(1);
    }

    let target_cmd: Option<&str> = if args.len() >= 2 {
        Some(&args[1])
    } else {
        None
    };
    let base_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // REPLを実行し、エラーがあれば表示して終了コード1で終わる
    if let Err(e) = run_repl(target_cmd, &base_path) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
