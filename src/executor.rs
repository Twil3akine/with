use std::env;
use std::process;

#[cfg(target_os = "windows")]
fn resolve_program(program: &str) -> String {
    match which::which(program) {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => program.to_string(),
    }
}

#[cfg(not(target_os = "windows"))]
fn resolve_program(program: &str) -> String {
    program.to_string()
}

// --- コマンド実行処理 ---
/// 指定されたプログラムを子プロセスとして実行する関数
/// 失敗しても親プロセス（このREPL）はクラッシュさせない
pub fn execute_child_process(program: &str, args: Vec<String>, current_context_prog: Option<&str>) {
    let program_path = resolve_program(program);

    let mut command = process::Command::new(program_path);
    command.args(args);

    // 現在のスタックを取得
    let parent_stack = env::var("WITH_CONTEXT_STACK").ok();

    // 新しいスタックを構築
    let new_stack = match parent_stack {
        Some(parent) => {
            let ctx = current_context_prog.unwrap_or("");
            format!("{}/{}", parent, ctx)
        }
        None => String::new(),
    };

    // 環境変数が空じゃないならセットする
    command.env("WITH_CONTEXT_STACK", new_stack);

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

// --- テスト ---
#[cfg(test)]
mod tests {
    use super::*;

    // Windows環境内のみでのテスト
    #[test]
    #[cfg(target_os = "windows")]
    fn test_resolve_program_windows_cmd() {
        // cmd.exe は必ずあるはず
        let res = resolve_program("cmd");
        // "C:\Windows\System32\cmd.exe" のようなフルパスになっているかチェック
        // (パスは環境によるので、とりあえず .exe で終わっているか確認など)
        assert!(res.to_lowercase().ends_with(".exe"));
        assert_ne!(res, "cmd");
    }

    // Windows環境で存在しないコマンドのテスト
    #[test]
    #[cfg(target_os = "windows")]
    fn test_resolve_program_windows_not_found() {
        let cmd = "non_existent_command_12345aaaaaaaa";
        let res = resolve_program(cmd);
        // 見つからない場合は入力がそのまま返る仕様
        assert_eq!(res, cmd);
    }

    // Linux/Mac環境でのテスト（変換しないことの確認）
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_resolve_program_unix_noop() {
        let cmd = "ls";
        let res = resolve_program(cmd);
        assert_eq!(res, cmd);
    }
}
