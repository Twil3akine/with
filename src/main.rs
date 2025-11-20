use rustyline::{
    Cmd, Completer, Editor, Helper, Hinter, KeyCode, Modifiers, Movement, Result, Validator,
    error::ReadlineError, highlight::Highlighter,
};
use shell_words;
use std::{
    borrow::Cow,
    env, eprintln, format,
    option::Option::None,
    path::{Path, PathBuf},
    println, process,
    result::Result::Ok,
};

// --- 定数定義 ---
// 終了判定に使うコマンドのリスト
const EXIT_COMMANDS: [&str; 4] = ["e", "q", "exit", "quit"];
// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_CYAN: &str = "\x1b[36m";
const STYLE_BOLD: &str = "\x1b[1m";
const STYLE_RESET: &str = "\x1b[0m"; // 色も太字も全部リセット

// プロンプトの装飾用マーカー（Highlighterでの検知にも使用）
const PROMPT_OPEN: &str = " [";
const PROMPT_CLOSE: &str = "]> ";

// --- Rustylineのヘルパー設定 ---
#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {}

// プロンプトの色付けロジックを実装
impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        // プロンプトが "cmd (dir)> " の形かチェックして色付け
        // "(" と ")> " で分割して場所を特定します
        if let (Some(start), Some(end)) = (prompt.find(PROMPT_OPEN), prompt.find(PROMPT_CLOSE)) {
            let cmd_part = &prompt[0..start];
            // PROMPT_OPENの長さ分ずらす
            let dir_part = &prompt[start + PROMPT_OPEN.len()..end];

            let styled = format!(
                "{}{}{}{} [{}{}{}]{}{}> ",
                STYLE_BOLD,
                COLOR_CYAN,
                cmd_part,
                STYLE_RESET, // Cmd
                COLOR_GREEN,
                dir_part,
                STYLE_RESET, // Dir
                STYLE_BOLD,  // Arrow
                STYLE_RESET
            );
            return Cow::Owned(styled);
        }

        // パースできなかったらそのまま返す
        Cow::Borrowed(prompt)
    }
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

/// 表示用のディレクトリ名を取得
/// base_path (起動時の場所) と同じなら "." を返す
fn get_display_dir(base_path: &Path) -> String {
    let current = env::current_dir().unwrap_or_default();

    if current == base_path {
        ".".to_string()
    } else {
        current
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string()
    }
}

/// 入力されたコマンドラインを解析して処理を振り分ける
fn handle_command(line: &str, target_cmd: &str) {
    let mut args = match shell_words::split(line) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error parsing command: {}", e);
            return;
        }
    };

    if args.is_empty() {
        execute_child_process(target_cmd, args);
        return;
    }

    let first_arg = &args[0];

    // 1. 脱出コマンド (!cmd)
    if first_arg.starts_with('!') {
        let is_attached = first_arg.len() > 1; // "!ls" vs "! ls"

        let program = if is_attached {
            // "!ls" -> "ls" (先頭の!を削除)
            args[0].remove(0);
            args[0].clone()
        } else {
            // "! ls" -> "ls" (先頭要素を捨てる)
            args.remove(0);
            // 削除後に要素がなければ何もしない
            if args.is_empty() {
                return;
            }
            args[0].clone()
        };

        // コマンド名を除いた引数リストを作成
        let program_args = if is_attached {
            args[1..].to_vec()
        } else {
            args[1..].to_vec()
        };

        execute_child_process(&program, program_args);
    }
    // 2. 内部コマンド (cd)
    else if first_arg == "cd" {
        let target = args.get(1).map(|s| s.as_str());
        match target {
            Some(path) => {
                if let Err(e) = env::set_current_dir(path) {
                    eprintln!("Failed to change directory: {}", e);
                }
            }
            None => eprintln!("Usage: cd <PATH>"),
        }
    }
    // 3. 通常実行 (Target Command)
    else {
        execute_child_process(target_cmd, args);
    }
}

// --- メインループ ---
/// REPL（対話型ループ）のメインロジック
fn run_repl(target_cmd: &str, base_path: &Path) -> Result<()> {
    // エディタの初期化
    let mut rl = Editor::<MyHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(MyHelper {}));

    // キーバインド設定: Escキーで入力行を全削除（Windowsライクな挙動）
    rl.bind_sequence(
        rustyline::KeyEvent(KeyCode::Esc, Modifiers::NONE),
        Cmd::Kill(Movement::WholeLine),
    );

    loop {
        let dir_name = get_display_dir(base_path);

        // プロンプトの文字列を作成（例: "git> "）
        let prompt = format!("{} [{}]> ", target_cmd, dir_name);

        // ユーザーの入力を待機
        let readline = rl.readline(&format!("{}", prompt));

        match readline {
            Ok(line) => {
                let line = line.trim();

                // 空行なら何もしないでループ先頭へ
                if !line.is_empty() {
                    // 履歴に追加（矢印キー上が使えるようになる）
                    rl.add_history_entry(line)?;
                }

                // 終了コマンドかどうかチェック
                if EXIT_COMMANDS.contains(&line) {
                    break;
                }

                handle_command(line, target_cmd);
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
    if args.len() != 2 {
        eprintln!("Usage: with <COMMAND>");
        process::exit(1);
    }

    let target_cmd = &args[1];
    let base_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // REPLを実行し、エラーがあれば表示して終了コード1で終わる
    if let Err(e) = run_repl(target_cmd, &base_path) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
