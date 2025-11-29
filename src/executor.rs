use std::process;
use which::which;

#[cfg(target_os = "windows")]
fn resolve_program(program: &str) -> String {
    match which(program) {
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
pub fn execute_child_process(program: &str, args: Vec<String>) {
    let program_path = resolve_program(program);

    let mut command = process::Command::new(program_path);
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
