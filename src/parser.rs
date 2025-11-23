use std::{
    fs,
    option::Option::{None, Some},
    path::Path,
};

#[derive(Debug, PartialEq)]
pub enum CommandAction {
    Execute { program: String, args: Vec<String> },
    ChangeDirectory(Option<String>),
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

/// ディレクトリ表示名の解決ロジック
/// current: 現在のディレクトリ, base: 起動時のディレクトリ
pub fn resolve_display_dir(current: &Path, base: &Path) -> Option<String> {
    if current == base {
        Some(".".to_string())
    } else {
        Some(
            current
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(".")
                .to_string(),
        )
    }
}

// --- Git branch 取得ロジック---
/// ファイルの中身からブランチ名またはハッシュを抽出する純粋関数
fn parse_git_head(content: &str) -> Option<String> {
    let content = content.trim();

    // "ref: refs/heads/main" の形式なら "main" を返す
    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        return Some(branch.to_string());
    }

    // Detached HEAD (ハッシュ値) の場合は先頭7文字を返す
    if content.len() >= 7 {
        return Some(content[..7].to_string());
    }

    None
}

/// カレントディレクトリから遡って .git/HEAD を探し、ブランチ名を返す
pub fn get_git_branch(cwd: &Path) -> Option<String> {
    let mut current = cwd;

    loop {
        let git_dir = current.join(".git");
        let head_path = git_dir.join("HEAD");

        if head_path.exists() {
            // HEADファイルを読み込む
            if let Ok(content) = fs::read_to_string(head_path) {
                return parse_git_head(&content);
            }
            return None;
        }

        match current.parent() {
            Some(p) => current = p,
            None => break,
        }
    }
    None
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

    // --- resolve_display_dir のテスト ---

    #[test]
    fn test_display_dir_same() {
        let base = std::path::PathBuf::from("/home/user/project");
        let current = std::path::PathBuf::from("/home/user/project");

        assert_eq!(resolve_display_dir(&current, &base), Some(".".to_string()));
    }

    #[test]
    fn test_display_dir_diff() {
        let base = std::path::PathBuf::from("/home/user/project");
        let current = std::path::PathBuf::from("/home/user/project/src");

        // "src" が返るはず
        assert_eq!(
            resolve_display_dir(&current, &base),
            Some("src".to_string())
        );
    }

    #[test]
    fn test_parse_git_head_branch() {
        let content = "ref: refs/heads/main\n";
        assert_eq!(parse_git_head(content), Some("main".to_string()));
    }

    #[test]
    fn test_parse_git_head_detached() {
        let content = "a1b2c3d4e5f67890abcdef1234567890abcdef12";
        assert_eq!(parse_git_head(content), Some("a1b2c3d".to_string()));
    }

    #[test]
    fn test_parse_git_head_invalid() {
        let content = "short";
        assert_eq!(parse_git_head(content), None);
    }
}
