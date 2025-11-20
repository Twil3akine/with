use rustyline::{
    Cmd, Completer, Editor, Helper, Hinter, KeyCode, Modifiers, Movement, Result, Validator,
    error::ReadlineError, highlight::Highlighter,
};
use shell_words;
use std::{
    borrow::Cow, env, eprintln, format, option::Option::None, println, process, result::Result::Ok,
};

// --- 定数定義 ---
// 終了判定に使うコマンドのリスト
const EXIT_COMMANDS: [&str; 4] = ["e", "q", "exit", "quit"];
// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_CYAN: &str = "\x1b[36m";
const STYLE_BOLD: &str = "\x1b[1m";

const STYLE_RESET: &str = "\x1b[0m"; // 色も太字も全部リセット

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
        if let Some(start_idx) = prompt.find(" [") {
            if let Some(end_idx) = prompt.find("]> ") {
                // パーツを切り出し
                let cmd_part = &prompt[0..start_idx]; // "cargo"
                let dir_part = &prompt[start_idx + 2..end_idx]; // "with" (" (" の2文字分ずらす)

                let styled = format!(
                    "{}{}{}{} [{}{}{}]{}{}> ",
                    STYLE_BOLD,
                    COLOR_CYAN,
                    cmd_part,
                    STYLE_RESET, // コマンド
                    COLOR_GREEN,
                    dir_part,
                    STYLE_RESET, // ディレクトリ
                    STYLE_BOLD,  // 最後の矢印を太字に
                    STYLE_RESET  // リセット
                );

                return Cow::Owned(styled);
            }
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

// --- メインループ ---
/// REPL（対話型ループ）のメインロジック
fn run_repl(target_cmd: &str, base_path: &str) -> Result<()> {
    // エディタの初期化
    let mut rl = Editor::<MyHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(MyHelper {}));

    // キーバインド設定: Escキーで入力行を全削除（Windowsライクな挙動）
    rl.bind_sequence(
        rustyline::KeyEvent(KeyCode::Esc, Modifiers::NONE),
        Cmd::Kill(Movement::WholeLine),
    );

    loop {
        // 現在のカレントディレクトリを取得
        let current_path = env::current_dir().unwrap();
        let dir_name = if current_path.to_str().unwrap() == base_path {
            "."
        } else {
            current_path.file_stem().unwrap().to_str().unwrap()
        };

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

                // 入力された文字列をスペース区切りでパース（引用符などを考慮）して実行
                match shell_words::split(line) {
                    Ok(mut args) => {
                        if args.is_empty() {
                            execute_child_process(target_cmd, args);
                        }
                        // '!'から始まる場合はエスケープモードに移行
                        else if args[0].starts_with('!') {
                            let separate_flg: bool = args[0].len() == 1;
                            if separate_flg {
                                args.remove(0);
                            }
                            // イテレータの最初を抜き出す
                            let cmd_with_bang = args.remove(0);
                            // '!'を除去
                            let tmp_cmd = if !separate_flg {
                                &cmd_with_bang[1..]
                            } else {
                                cmd_with_bang.as_str()
                            };

                            execute_child_process(tmp_cmd, args);
                        } else if args[0] == "cd" {
                            match args.get(1) {
                                Some(path) => {
                                    // ディレクトリ移動を実行
                                    if let Err(e) = env::set_current_dir(path) {
                                        // 指定したディレクトリがなかった場合、警告が表示される
                                        eprintln!("Failed to change directory: {}", e);
                                    }
                                }
                                None => {
                                    // 引数がないと警告が表示される
                                    eprintln!("Usage: cd <PATH>");
                                }
                            }
                        } else {
                            execute_child_process(target_cmd, args);
                        }
                    }
                    Err(e) => eprintln!("Error parsing command: {}", e),
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
    if args.len() != 2 {
        eprintln!("Usage: with <COMMAND>");
        process::exit(1);
    }

    let target_cmd = &args[1];
    let base_path = env::current_dir().unwrap();
    let base_path = base_path.to_str().unwrap();

    // REPLを実行し、エラーがあれば表示して終了コード1で終わる
    if let Err(e) = run_repl(target_cmd, base_path) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
