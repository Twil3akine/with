use std::option::Option::{None, Some};

#[derive(Debug, PartialEq)]
pub enum CommandAction {
    Execute { program: String, args: Vec<String> },
    ChangeDirectory(Option<String>),
    Help,
    Clear(Vec<String>),
    Pwd(Vec<String>),
    History,
    DoNothing,
    Exit,
    Error(String),
}

#[derive(Clone)]
pub struct TargetContext {
    pub program: String,
    pub args: Vec<String>,
}

// 終了判定に使うコマンドのリスト
const EXIT_COMMANDS: [&str; 4] = ["e", "q", "exit", "quit"];

/// 入力行とターゲットコマンドを受け取り、アクションを返す
pub fn parse_cmd(line: &str, context: Option<&TargetContext>) -> CommandAction {
    let line = line.trim();

    // Windows対応: 表示は '\' (バックスラッシュ) だが、
    // shell-words に渡す前に内部的に '/' (スラッシュ) に置換する。
    #[cfg(windows)]
    let line_owned = line.replace('\\', "/");

    #[cfg(windows)]
    let line = line_owned.as_str();

    // 終了コマンドかどうかチェック
    if EXIT_COMMANDS.contains(&line) {
        return CommandAction::Exit;
    }

    // 引数を分割
    let mut args = match shell_words::split(line) {
        Ok(a) => a,
        Err(e) => return CommandAction::Error(e.to_string()),
    };

    if args.is_empty() {
        if let Some(ctx) = context {
            return CommandAction::Execute {
                program: ctx.program.clone(),
                args: ctx.args.clone(),
            };
        }
        return CommandAction::DoNothing;
    }

    // 先頭の要素（コマンド名候補）を取得
    let first_arg: &str = &args[0];

    match first_arg {
        // --- 内部コマンド (Built-in) ---
        "cd" => {
            let target = if args.len() > 1 {
                Some(args[1].to_string())
            } else {
                None
            };
            CommandAction::ChangeDirectory(target)
        }
        "clear" | "cls" => {
            args.remove(0);
            CommandAction::Clear(args)
        }
        "pwd" => {
            args.remove(0);
            CommandAction::Pwd(args)
        }
        "history" => CommandAction::History,
        "help" => CommandAction::Help,

        // --- 脱出コマンド (!cmd) ---
        s if s.starts_with('!') => {
            let program;
            if s.len() > 1 {
                program = s[1..].to_string();
                args.remove(0);
            } else {
                args.remove(0);
                if args.is_empty() {
                    return CommandAction::DoNothing;
                }
                program = args.remove(0);
            }
            CommandAction::Execute { program, args }
        }

        // --- 通常実行 ---
        _ => {
            if let Some(ctx) = context {
                let mut final_args = ctx.args.clone();
                final_args.append(&mut args);

                CommandAction::Execute {
                    program: ctx.program.clone(),
                    args: final_args,
                }
            } else {
                let program = args.remove(0);
                CommandAction::Execute { program, args }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ヘルパー関数 ---
    // CommandAction::Execute の中身（プログラム名と引数）を簡単に検証するための関数
    fn assert_execute(action: CommandAction, expected_prog: &str, expected_args: &[&str]) {
        match action {
            CommandAction::Execute { program, args } => {
                assert_eq!(program, expected_prog);
                assert_eq!(args, expected_args);
            }
            _ => panic!("Expected Execute, got {:?}", action),
        }
    }

    // --- 基本動作テスト ---

    /// ターゲットコマンド指定時の基本動作
    /// 例: `with git` 起動中に `status` と入力 -> `git status` が実行されるか
    #[test]
    fn test_target_cmd_basic() {
        let action = parse_cmd("status", Some("git"));
        assert_execute(action, "git", &["status"]);
    }

    /// ターゲットコマンドに引数がある場合
    /// 例: `commit -m "msg"` -> `git commit -m "msg"` と分解されるか
    #[test]
    fn test_target_cmd_with_args() {
        let action = parse_cmd("commit -m \"msg\"", Some("git"));
        assert_execute(action, "git", &["commit", "-m", "msg"]);
    }

    /// ターゲットコマンドなし（with単体起動）の場合
    /// 例: `ls -al` -> そのまま `ls` コマンドとして実行されるか
    #[test]
    fn test_no_target_basic() {
        let action = parse_cmd("ls -al", None);
        assert_execute(action, "ls", &["-al"]);
    }

    // --- 脱出コマンド (!cmd) テスト ---

    /// "!" とコマンドがくっついている場合
    /// 例: `!ls` -> `git` ではなく `ls` が実行されるか
    #[test]
    fn test_escape_command_attached() {
        let action = parse_cmd("!ls -h", Some("git"));
        assert_execute(action, "ls", &["-h"]);
    }

    /// "!" とコマンドが離れている場合
    /// 例: `! ls` -> スペースがあっても正しく `ls` が認識されるか
    #[test]
    fn test_escape_command_detached() {
        let action = parse_cmd("! ls -h", Some("git"));
        assert_execute(action, "ls", &["-h"]);
    }

    // --- 内部コマンド (cd) テスト ---

    /// ディレクトリ移動 (cd)
    /// 例: `cd src` -> ChangeDirectory アクションが生成されるか
    #[test]
    fn test_cd_command() {
        let action = parse_cmd("cd src", Some("git"));
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "src"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    /// 引数なしの cd
    /// 例: `cd` -> ChangeDirectory(None) (ホームディレクトリ移動などの意味) になるか
    #[test]
    fn test_cd_empty() {
        let action = parse_cmd("cd", Some("git"));
        match action {
            CommandAction::ChangeDirectory(None) => {} // OK
            _ => panic!("Expected ChangeDirectory(None), got {:?}", action),
        }
    }

    // --- 終了コマンド テスト ---

    /// 終了コマンドのエイリアス確認
    /// `exit`, `q` などがすべて Exit アクションになるか
    #[test]
    fn test_exit_commands() {
        assert_eq!(parse_cmd("exit", Some("git")), CommandAction::Exit);
        assert_eq!(parse_cmd("q", None), CommandAction::Exit);
    }

    // --- 空入力のハンドリング ---

    /// ターゲット指定時の空入力
    /// 例: `with git` で空エンター -> `git` 単体（ヘルプ表示など）を実行するか
    #[test]
    fn test_empty_input_executes_target() {
        let action = parse_cmd("", Some("git"));
        assert_execute(action, "git", &[]);
    }

    /// ターゲットなし時の空入力
    /// 例: `with` 単体で空エンター -> 何もしない (DoNothing) か
    #[test]
    fn test_empty_input_no_target() {
        let action = parse_cmd("", None);
        assert_eq!(action, CommandAction::DoNothing);
    }

    // --- パースのエラー処理・特殊ケース ---

    /// クォートの閉じ忘れ
    /// 例: `echo "hello` -> エラーとして処理されるか
    #[test]
    fn test_unclosed_quote() {
        let action = parse_cmd("echo \"hello", None);
        match action {
            CommandAction::Error(_) => {} // OK
            _ => panic!("Expected Error due to unclosed quote, got {:?}", action),
        }
    }

    /// クォート内のスペース処理
    /// 例: `"fix bug"` が1つの引数として扱われるか
    #[test]
    fn test_quoted_arguments_with_spaces() {
        let action = parse_cmd("commit -m \"fix bug\"", Some("git"));
        assert_execute(action, "git", &["commit", "-m", "fix bug"]);
    }

    /// 連続スペースの正規化
    /// 例: `ls    -a` -> `ls`, `-a` と正しく分割されるか
    #[test]
    fn test_multiple_spaces_normalization() {
        let action = parse_cmd("  ls    -a      -l  ", None);
        assert_execute(action, "ls", &["-a", "-l"]);
    }

    /// "!" だけ入力された場合
    /// 無効な入力として無視 (DoNothing) されるか
    #[test]
    fn test_escape_char_only() {
        let action = parse_cmd("!", Some("git"));
        assert_eq!(action, CommandAction::DoNothing);
    }

    /// "!" とコマンドの間に大量のスペースがある場合
    #[test]
    fn test_escape_detached_multiple_spaces() {
        let action = parse_cmd("!    ls -h", Some("git"));
        assert_execute(action, "ls", &["-h"]);
    }

    /// cd に引数が多すぎる場合
    /// 例: `cd dir1 dir2` -> 最初の引数 `dir1` だけが採用されるか
    #[test]
    fn test_cd_with_too_many_args() {
        let action = parse_cmd("cd dir1 dir2", None);
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "dir1"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    /// シングルクォートの処理
    /// 例: `'foo bar'` もダブルクォート同様に1つの引数になるか
    #[test]
    fn test_single_quote_handling() {
        let action = parse_cmd("echo 'foo bar'", None);
        assert_execute(action, "echo", &["foo bar"]);
    }

    // --- 新規実装コマンド (Clear / Help) テスト ---

    /// clear コマンド (引数なし)
    /// Clear アクションになり、引数リストが空か
    #[test]
    fn test_cmd_clear_no_args() {
        let action = parse_cmd("clear", None);
        match action {
            CommandAction::Clear(args) => assert!(args.is_empty()),
            _ => panic!("Expected Clear, got {:?}", action),
        }
    }

    /// clear コマンド (引数あり)
    /// 例: `clear -x` -> 引数が保持されているか
    #[test]
    fn test_cmd_clear_with_args() {
        let action = parse_cmd("clear -x", None);
        match action {
            CommandAction::Clear(args) => assert_eq!(args, vec!["-x"]),
            _ => panic!("Expected Clear with args, got {:?}", action),
        }
    }

    /// cls (Windowsエイリアス)
    /// `cls` と打っても `Clear` アクションになるか
    #[test]
    fn test_cmd_cls_windows_alias() {
        let action = parse_cmd("cls", None);
        match action {
            CommandAction::Clear(args) => assert!(args.is_empty()),
            _ => panic!("Expected Clear(cls), got {:?}", action),
        }
    }

    /// help コマンド
    /// Help アクションになるか
    #[test]
    fn test_cmd_help() {
        let action = parse_cmd("help", None);
        assert_eq!(action, CommandAction::Help);
    }

    /// help コマンド (引数無視)
    /// `help me` と打っても引数は無視され、単なる Help になるか
    #[test]
    fn test_cmd_help_ignores_args() {
        let action = parse_cmd("help me", None);
        assert_eq!(action, CommandAction::Help);
    }

    // --- HISTORY コマンドのテスト ---

    /// history 単体
    /// History アクションが正しく生成されるか
    #[test]
    fn test_cmd_history_basic() {
        let action = parse_cmd("history", None);
        assert_eq!(action, CommandAction::History);
    }

    /// history ターゲット指定時の優先度
    /// 例: "with git" 状態でも、"history" と打てば内部履歴を表示すべき
    /// ("git history" というサブコマンドとしては解釈されない)
    #[test]
    fn test_cmd_history_priority() {
        let action = parse_cmd("history", Some("git"));
        assert_eq!(action, CommandAction::History);
    }

    /// history 引数あり
    /// 現状の実装では引数（"history 10"など）は無視して、
    /// Historyアクション（全履歴表示）になる仕様を確認
    #[test]
    fn test_cmd_history_ignores_args() {
        let action = parse_cmd("history 10", None);
        assert_eq!(action, CommandAction::History);
    }

    // --- PWD コマンドのテスト ---

    /// pwd 単体
    /// 引数なしの Pwd アクションになるか
    #[test]
    fn test_cmd_pwd_basic() {
        let action = parse_cmd("pwd", None);
        match action {
            CommandAction::Pwd(args) => assert!(args.is_empty()),
            _ => panic!("Expected Pwd, got {:?}", action),
        }
    }

    /// pwd 引数あり (-L, -P など)
    /// 引数が正しくベクタに格納され、実行時に渡されるようになっているか
    #[test]
    fn test_cmd_pwd_with_args() {
        // "pwd -L" -> logical path
        let action = parse_cmd("pwd -L", None);
        match action {
            CommandAction::Pwd(args) => assert_eq!(args, vec!["-L"]),
            _ => panic!("Expected Pwd with args, got {:?}", action),
        }
    }

    /// pwd ターゲット指定時の優先度
    /// 例: "with git" 状態でも "pwd" は内部コマンドとして処理されるべき
    /// ("git pwd" にはならない)
    #[test]
    fn test_cmd_pwd_priority() {
        let action = parse_cmd("pwd", Some("git"));
        match action {
            CommandAction::Pwd(_) => {} // OK
            _ => panic!("Expected Pwd action (priority check), got {:?}", action),
        }
    }

    // --- OS依存処理 (Windowsパス置換) テスト ---

    /// Windows環境でのパス置換テスト
    /// `\` が `/` に置換され、文字消滅が防げているか
    #[test]
    #[cfg(windows)]
    fn test_windows_path_conversion() {
        // Input: "add src\main.rs"
        // Expected: git add src/main.rs
        let action = parse_cmd("add src\\main.rs", Some("git"));
        assert_execute(action, "git", &["add", "src/main.rs"]);
    }

    /// 非Windows環境でのパス処理テスト
    /// `\` はエスケープ文字として扱われ、文字が消える（標準仕様通り）か
    #[test]
    #[cfg(not(windows))]
    fn test_unix_path_handling() {
        // Input: "add src\main.rs" -> "srcmain.rs"
        let action = parse_cmd("add src\\main.rs", Some("git"));
        assert_execute(action, "git", &["add", "srcmain.rs"]);
    }
}
