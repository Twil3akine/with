use rustyline::{
    error::ReadlineError,
    highlight::Highlighter,
    Cmd,
    Completer,
    Editor,
    Helper,
    Hinter,
    KeyCode,
    Modifiers,
    Movement,
    Result,
    Validator,
};
use shell_words;
use std::{
    borrow::Cow,
    env,
    eprintln,
    format,
    println,
    process,
    result::Result::Ok,
};

// --- 定数定義 ---
// 終了判定に使うコマンドのリスト
const EXIT_COMMANDS: [&str; 4] = ["e", "q", "exit", "quit"];
// プロンプトの色付け用 (シアン)
const COLOR_CYAN: &str = "\x1b[36m";
// 色のリセット用 (デフォルト色に戻す)
const COLOR_DEFAULT: &str = "\x1b[39m";

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
        if prompt.contains(">") {
            // ">" を "（デフォルト色）>" に置換して、入力文字が水色にならないようにする
            let styled = prompt.replace(">", &format!("{}>", COLOR_DEFAULT));
            // プロンプト全体を水色（CYAN）で囲む
            Cow::Owned(format!("{}{}", COLOR_CYAN, styled))
        } else {
            Cow::Borrowed(prompt)
        }
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
fn run_repl(target_cmd: &str) -> Result<()> {
    // エディタの初期化
    let mut rl = Editor::<MyHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(MyHelper {}));

    // キーバインド設定: Escキーで入力行を全削除（Windowsライクな挙動）
    rl.bind_sequence(
        rustyline::KeyEvent(KeyCode::Esc, Modifiers::NONE),
        Cmd::Kill(Movement::WholeLine),
    );

    // プロンプトの文字列を作成（例: "git> "）
    let prompt = format!("{}> ", target_cmd);

    loop {
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
                        // '!'から始まる場合はエスケープモードに移行
                        let separate_flg: bool = args[0].len() == 1;
                        if args[0].starts_with('!') {
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

    // REPLを実行し、エラーがあれば表示して終了コード1で終わる
    if let Err(e) = run_repl(target_cmd) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
