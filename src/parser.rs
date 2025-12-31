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
    ExitAll,
    Error(String),
}

#[derive(Clone)]
pub struct TargetContext {
    pub program: String,
    pub args: Vec<String>,
}

/// 入力行とターゲットコマンドを受け取り、アクションを返す
pub fn parse_cmd(line: &str, context: Option<&TargetContext>) -> CommandAction {
    let line = line.trim();

    // Windows対応: 表示は '\' (バックスラッシュ) だが、
    // shell-words に渡す前に内部的に '/' (スラッシュ) に置換する。
    #[cfg(windows)]
    let line_owned = line.replace('\\', "/");

    #[cfg(windows)]
    let line = line_owned.as_str();

    // 終了コマンドの判定
    match line {
        "exit" | "e" => return CommandAction::ExitAll,
        "quit" | "q" => return CommandAction::Exit,
        _ => {}
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

    // テスト用のコンテキスト作成ヘルパー
    // 例: create_ctx("git", &["remote"]) -> TargetContext { program: "git", args: ["remote"] }
    fn create_ctx(program: &str, args: &[&str]) -> Option<TargetContext> {
        Some(TargetContext {
            program: program.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        })
    }

    // CommandAction::Execute の中身検証ヘルパー
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

    /// ターゲットコマンド指定時の基本動作 (引数なしコンテキスト)
    /// 例: `with git` 起動中に `status` と入力 -> `git status`
    #[test]
    fn test_target_cmd_basic() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("status", ctx.as_ref());
        assert_execute(action, "git", &["status"]);
    }

    /// ターゲットコマンドに引数がある場合
    /// 例: `commit -m "msg"` -> `git commit -m "msg"`
    #[test]
    fn test_target_cmd_with_args() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("commit -m \"msg\"", ctx.as_ref());
        assert_execute(action, "git", &["commit", "-m", "msg"]);
    }

    /// 例: `with git remote` (ctx args=["remote"]) + 入力 `-v` -> `git remote -v`
    #[test]
    fn test_target_context_concatenation() {
        let ctx = create_ctx("git", &["remote"]);
        let action = parse_cmd("-v", ctx.as_ref());
        // args が ["remote", "-v"] に結合されていることを確認
        assert_execute(action, "git", &["remote", "-v"]);
    }

    /// 例: `with docker compose exec` + 入力 `app bash`
    #[test]
    fn test_target_context_multiple_args() {
        let ctx = create_ctx("docker", &["compose", "exec"]);
        let action = parse_cmd("app bash", ctx.as_ref());
        assert_execute(action, "docker", &["compose", "exec", "app", "bash"]);
    }

    /// ターゲットコマンドなし（with単体起動）の場合
    #[test]
    fn test_no_target_basic() {
        let action = parse_cmd("ls -al", None);
        assert_execute(action, "ls", &["-al"]);
    }

    // --- 脱出コマンド (!cmd) テスト ---

    #[test]
    fn test_escape_command_attached() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("!ls -h", ctx.as_ref());
        assert_execute(action, "ls", &["-h"]);
    }

    #[test]
    fn test_escape_command_detached() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("! ls -h", ctx.as_ref());
        assert_execute(action, "ls", &["-h"]);
    }

    // --- 内部コマンド (cd) テスト ---

    #[test]
    fn test_cd_command() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("cd src", ctx.as_ref());
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "src"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    #[test]
    fn test_cd_empty() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("cd", ctx.as_ref());
        match action {
            CommandAction::ChangeDirectory(None) => {} // OK
            _ => panic!("Expected ChangeDirectory(None), got {:?}", action),
        }
    }

    // --- 終了コマンド テスト ---

    #[test]
    fn test_exit_commands() {
        let ctx = create_ctx("git", &[]);
        assert_eq!(parse_cmd("exit", ctx.as_ref()), CommandAction::Exit);
        assert_eq!(parse_cmd("q", None), CommandAction::Exit);
    }

    // --- 空入力のハンドリング ---

    /// ターゲット指定時の空入力
    /// コンテキストの args もそのまま実行されるべき
    #[test]
    fn test_empty_input_executes_target() {
        let ctx = create_ctx("git", &["status"]);
        let action = parse_cmd("", ctx.as_ref());
        // "git status" が実行される
        assert_execute(action, "git", &["status"]);
    }

    #[test]
    fn test_empty_input_no_target() {
        let action = parse_cmd("", None);
        assert_eq!(action, CommandAction::DoNothing);
    }

    // --- パースのエラー処理・特殊ケース ---

    #[test]
    fn test_unclosed_quote() {
        let action = parse_cmd("echo \"hello", None);
        match action {
            CommandAction::Error(_) => {} // OK
            _ => panic!("Expected Error due to unclosed quote, got {:?}", action),
        }
    }

    #[test]
    fn test_quoted_arguments_with_spaces() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("commit -m \"fix bug\"", ctx.as_ref());
        assert_execute(action, "git", &["commit", "-m", "fix bug"]);
    }

    #[test]
    fn test_multiple_spaces_normalization() {
        let action = parse_cmd("  ls    -a      -l  ", None);
        assert_execute(action, "ls", &["-a", "-l"]);
    }

    #[test]
    fn test_escape_char_only() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("!", ctx.as_ref());
        assert_eq!(action, CommandAction::DoNothing);
    }

    #[test]
    fn test_escape_detached_multiple_spaces() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("!    ls -h", ctx.as_ref());
        assert_execute(action, "ls", &["-h"]);
    }

    #[test]
    fn test_cd_with_too_many_args() {
        let action = parse_cmd("cd dir1 dir2", None);
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "dir1"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    #[test]
    fn test_single_quote_handling() {
        let action = parse_cmd("echo 'foo bar'", None);
        assert_execute(action, "echo", &["foo bar"]);
    }

    // --- 新規実装コマンド (Clear / Help) テスト ---

    #[test]
    fn test_cmd_clear_no_args() {
        let action = parse_cmd("clear", None);
        match action {
            CommandAction::Clear(args) => assert!(args.is_empty()),
            _ => panic!("Expected Clear, got {:?}", action),
        }
    }

    #[test]
    fn test_cmd_clear_with_args() {
        let action = parse_cmd("clear -x", None);
        match action {
            CommandAction::Clear(args) => assert_eq!(args, vec!["-x"]),
            _ => panic!("Expected Clear with args, got {:?}", action),
        }
    }

    #[test]
    fn test_cmd_cls_windows_alias() {
        let action = parse_cmd("cls", None);
        match action {
            CommandAction::Clear(args) => assert!(args.is_empty()),
            _ => panic!("Expected Clear(cls), got {:?}", action),
        }
    }

    #[test]
    fn test_cmd_help() {
        let action = parse_cmd("help", None);
        assert_eq!(action, CommandAction::Help);
    }

    #[test]
    fn test_cmd_help_ignores_args() {
        let action = parse_cmd("help me", None);
        assert_eq!(action, CommandAction::Help);
    }

    // --- HISTORY / PWD コマンドのテスト ---

    #[test]
    fn test_cmd_history_basic() {
        let action = parse_cmd("history", None);
        assert_eq!(action, CommandAction::History);
    }

    #[test]
    fn test_cmd_history_priority() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("history", ctx.as_ref());
        assert_eq!(action, CommandAction::History);
    }

    #[test]
    fn test_cmd_history_ignores_args() {
        let action = parse_cmd("history 10", None);
        assert_eq!(action, CommandAction::History);
    }

    #[test]
    fn test_cmd_pwd_basic() {
        let action = parse_cmd("pwd", None);
        match action {
            CommandAction::Pwd(args) => assert!(args.is_empty()),
            _ => panic!("Expected Pwd, got {:?}", action),
        }
    }

    #[test]
    fn test_cmd_pwd_with_args() {
        let action = parse_cmd("pwd -L", None);
        match action {
            CommandAction::Pwd(args) => assert_eq!(args, vec!["-L"]),
            _ => panic!("Expected Pwd with args, got {:?}", action),
        }
    }

    #[test]
    fn test_cmd_pwd_priority() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("pwd", ctx.as_ref());
        match action {
            CommandAction::Pwd(_) => {} // OK
            _ => panic!("Expected Pwd action, got {:?}", action),
        }
    }

    // --- OS依存処理 (Windowsパス置換) テスト ---

    #[test]
    #[cfg(windows)]
    fn test_windows_path_conversion() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("add src\\main.rs", ctx.as_ref());
        assert_execute(action, "git", &["add", "src/main.rs"]);
    }

    #[test]
    #[cfg(not(windows))]
    fn test_unix_path_handling() {
        let ctx = create_ctx("git", &[]);
        let action = parse_cmd("add src\\main.rs", ctx.as_ref());
        assert_execute(action, "git", &["add", "srcmain.rs"]);
    }
}
