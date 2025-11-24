use std::option::Option::{None, Some};

#[derive(Debug, PartialEq)]
pub enum CommandAction {
    Execute { program: String, args: Vec<String> },
    ChangeDirectory(Option<String>),
    Help,
    Clear(Vec<String>),
    DoNothing,
    Exit,
    Error(String),
}

// 終了判定に使うコマンドのリスト
const EXIT_COMMANDS: [&str; 4] = ["e", "q", "exit", "quit"];

/// 入力行とターゲットコマンドを受け取り、アクションを返す
pub fn parse_cmd(line: &str, target_cmd: Option<&str>) -> CommandAction {
    let line = line.trim();

    // 終了コマンドかどうかチェック
    if EXIT_COMMANDS.contains(&line) {
        return CommandAction::Exit;
    }

    let mut args = match shell_words::split(line) {
        Ok(a) => a,
        Err(e) => return CommandAction::Error(e.to_string()),
    };

    if args.is_empty() {
        if let Some(target_cmd) = target_cmd {
            return CommandAction::Execute {
                program: target_cmd.to_string(),
                args,
            };
        }
        return CommandAction::DoNothing;
    }

    let first_arg = &args[0];

    // 1. 脱出コマンド (!cmd)
    if let Some(first_arg) = first_arg.strip_prefix('!') {
        let program;
        if first_arg.len() > 1 {
            // "!ls" -> "ls"
            program = first_arg.to_string();
            args.remove(0);
        } else {
            // "! ls" -> "ls" (先頭要素 "!" を捨てる)
            args.remove(0);
            if args.is_empty() {
                return CommandAction::DoNothing;
            }
            program = args.remove(0);
        }

        return CommandAction::Execute {
            program,
            args, // 残りの引数
        };
    }
    // 2. 内部コマンド (cd)
    if first_arg == "cd" {
        let target = if args.len() > 1 {
            Some(args[1].clone())
        } else {
            None
        };
        CommandAction::ChangeDirectory(target)
    }
    // 3. 通常実行 (Target Command)
    else {
        match target_cmd {
            Some(target_cmd) => CommandAction::Execute {
                program: target_cmd.to_string(),
                args,
            },
            None => {
                let program = args.remove(0);
                CommandAction::Execute { program, args }
            }
        }
    }
}

// テストコード
#[cfg(test)]
mod tests {
    use super::*;

    // ヘルパー: executeの結果を検証しやすくする
    fn assert_execute(action: CommandAction, expected_prog: &str, expected_args: &[&str]) {
        match action {
            CommandAction::Execute { program, args } => {
                assert_eq!(program, expected_prog);
                assert_eq!(args, expected_args);
            }
            _ => panic!("Expected Execute, got {:?}", action),
        }
    }

    #[test]
    fn test_target_cmd_basic() {
        // with git
        // input status
        let action = parse_cmd("status", Some("git"));
        assert_execute(action, "git", &["status"]);
    }

    #[test]
    fn test_target_cmd_with_args() {
        // with git
        // input: commit -m "msg"
        let action = parse_cmd("commit -m \"msg\"", Some("git"));
        assert_execute(action, "git", &["commit", "-m", "msg"]);
    }

    #[test]
    fn test_no_target_basic() {
        // with (単体)
        // input: ls -la
        let action = parse_cmd("ls -al", None);
        assert_execute(action, "ls", &["-al"]);
    }

    #[test]
    fn test_escape_command_attached() {
        // input: !ls
        let action = parse_cmd("!ls -h", Some("git"));
        // git ではなく ls が実行されるべき
        assert_execute(action, "ls", &["-h"]);
    }

    #[test]
    fn test_escape_command_detached() {
        // input: ! ls
        let action = parse_cmd("! ls -h", Some("git"));
        assert_execute(action, "ls", &["-h"]);
    }

    #[test]
    fn test_cd_command() {
        // input: cd src
        let action = parse_cmd("cd src", Some("git"));
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "src"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    #[test]
    fn test_cd_empty() {
        // input: cd
        let action = parse_cmd("cd", Some("git"));
        match action {
            CommandAction::ChangeDirectory(None) => {} // OK
            _ => panic!("Expected ChangeDirectory(None), got {:?}", action),
        }
    }

    #[test]
    fn test_exit_commands() {
        assert_eq!(parse_cmd("exit", Some("git")), CommandAction::Exit);
        assert_eq!(parse_cmd("q", None), CommandAction::Exit);
    }

    #[test]
    fn test_empty_input_executes_target() {
        // with git
        // input: "" (空行)
        let action = parse_cmd("", Some("git"));
        assert_execute(action, "git", &[]);
    }

    // 仕様変更: target_cmdがない状態で空入力をすると、何もしない（従来通り）
    #[test]
    fn test_empty_input_no_target() {
        // with (単体起動)
        // input: "" (空行)
        let action = parse_cmd("", None);
        assert_eq!(action, CommandAction::DoNothing);
    }

    #[test]
    fn test_unclosed_quote() {
        // クォートが閉じられていない場合のエラーハンドリング
        // input: echo "hello
        let action = parse_cmd("echo \"hello", None);
        match action {
            CommandAction::Error(_) => {} // OK (エラーになるべき)
            _ => panic!("Expected Error due to unclosed quote, got {:?}", action),
        }
    }

    #[test]
    fn test_quoted_arguments_with_spaces() {
        // スペースを含む引数が正しく1つの引数として扱われるか
        // with git
        // input: commit -m "fix bug"
        let action = parse_cmd("commit -m \"fix bug\"", Some("git"));
        assert_execute(action, "git", &["commit", "-m", "fix bug"]);
    }

    #[test]
    fn test_multiple_spaces_normalization() {
        // 連続するスペースが無視され、正しくパースされるか
        // input:   ls    -a      -l
        let action = parse_cmd("  ls    -a      -l  ", None);
        assert_execute(action, "ls", &["-a", "-l"]);
    }

    #[test]
    fn test_escape_char_only() {
        // "!" だけ入力された場合
        // (! ls のつもりで ls を書き忘れた場合など)
        let action = parse_cmd("!", Some("git"));
        // 引数がなくなるため DoNothing になるはず
        assert_eq!(action, CommandAction::DoNothing);
    }

    #[test]
    fn test_escape_detached_multiple_spaces() {
        // "!    ls" のようにスペースが多い場合
        let action = parse_cmd("!    ls -h", Some("git"));
        assert_execute(action, "ls", &["-h"]);
    }

    #[test]
    fn test_cd_with_too_many_args() {
        // cd コマンドに引数が多すぎる場合、最初の引数だけ採用するか確認
        // input: cd dir1 dir2
        let action = parse_cmd("cd dir1 dir2", None);
        match action {
            CommandAction::ChangeDirectory(Some(path)) => assert_eq!(path, "dir1"),
            _ => panic!("Expected ChangeDirectory, got {:?}", action),
        }
    }

    #[test]
    fn test_single_quote_handling() {
        // シングルクォートの扱い
        // input: echo 'foo bar'
        let action = parse_cmd("echo 'foo bar'", None);
        assert_execute(action, "echo", &["foo bar"]);
    }
}
