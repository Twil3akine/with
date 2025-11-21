use shell_words;
use std::{
    option::Option::{None, Some},
    path::Path,
};

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
    if first_arg.starts_with('!') {
        let program;
        if first_arg.len() > 1 {
            // "!ls" -> "ls"
            program = first_arg[1..].to_string();
            // args[0] は "!ls" なので、これをプログラム名として使うわけにはいかないが、
            // execute用に args 全体を再構成する必要がある。
            // ここではシンプルに「プログラム名」と「残りの引数」を抽出する。
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
        return CommandAction::ChangeDirectory(target);
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
        None
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
