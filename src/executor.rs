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

/// 次のプロセスに渡すスタック文字列を計算する純粋関数
///
/// * `parent_stack`: 親プロセスから受け取ったスタック (例: "git", "")。NoneならRoot。
/// * `current_ctx`: 現在実行中のコンテキスト (例: "cargo")。Noneならwith単体。
fn compute_next_stack(parent_stack: Option<&str>, current_ctx: Option<&str>) -> String {
    match parent_stack {
        Some(parent) => {
            // 既に親がいる場合
            if let Some(ctx) = current_ctx {
                // 親スタック + "/" + 自分 (例: "git" + "cargo" -> "git/cargo")
                // 親が空文字("")の場合も "/cargo" となり、空階層が表現される
                format!("{}/{}", parent, ctx)
            } else {
                // 自分はコンテキストなし (例: "git" + None -> "git/")
                format!("{}/", parent)
            }
        }
        None => {
            // 親がいない (Root) 場合
            if let Some(ctx) = current_ctx {
                // 自分のコンテキストを起点にする (例: "git")
                ctx.to_string()
            } else {
                // 自分もなしなら空文字 (例: "")
                // これにより、子は「親はいるが空(Root直下)」と認識できる
                String::new()
            }
        }
    }
}

// --- コマンド実行処理 ---
/// 指定されたプログラムを子プロセスとして実行する関数
pub fn execute_child_process(program: &str, args: Vec<String>, current_context_prog: Option<&str>) {
    let program_path = resolve_program(program);

    let mut command = process::Command::new(program_path);
    command.args(args);

    // 現在のスタックを取得
    let parent_stack_opt = env::var("WITH_CONTEXT_STACK").ok();
    let parent_stack_str = parent_stack_opt.as_deref();

    // 次のスタックを計算
    let new_stack = compute_next_stack(parent_stack_str, current_context_prog);

    // 環境変数をセット
    command.env("WITH_CONTEXT_STACK", new_stack);

    // spawn() でプロセスを開始
    match command.spawn() {
        Ok(mut child) => {
            // wait() で子プロセスの終了を待機する
            match child.wait() {
                Ok(status) => {
                    // 子プロセスが「全終了(127)」で死んだ場合、自分も後を追う
                    if let Some(code) = status.code()
                        && code == 127
                    {
                        process::exit(127);
                    }
                }
                Err(e) => {
                    eprintln!("Error waiting for process: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to execute command '{}': {}", program, e);
        }
    }
}

// --- テスト ---
#[cfg(test)]
mod tests {
    use super::*;

    // --- compute_next_stack のテスト ---

    #[test]
    fn test_stack_root_with_context() {
        // Root: with git -> child stack should be "git"
        let res = compute_next_stack(None, Some("git"));
        assert_eq!(res, "git");
    }

    #[test]
    fn test_stack_root_no_context() {
        // Root: with -> child stack should be "" (Empty Root)
        let res = compute_next_stack(None, None);
        assert_eq!(res, "");
    }

    #[test]
    fn test_stack_nested_context() {
        // L1: git -> rc cargo -> "git/cargo"
        let res = compute_next_stack(Some("git"), Some("cargo"));
        assert_eq!(res, "git/cargo");
    }

    #[test]
    fn test_stack_nested_no_context() {
        // L1: git -> rc -> "git/"
        let res = compute_next_stack(Some("git"), None);
        assert_eq!(res, "git/");
    }

    #[test]
    fn test_stack_from_empty_root_with_context() {
        // L1: "" (from Root 'with') -> rc git -> "/git"
        let res = compute_next_stack(Some(""), Some("git"));
        assert_eq!(res, "/git");
    }

    #[test]
    fn test_stack_from_empty_root_no_context() {
        // L1: "" (from Root 'with') -> rc -> "/"
        let res = compute_next_stack(Some(""), None);
        assert_eq!(res, "/");
    }

    #[test]
    fn test_stack_deep_nesting() {
        // L2: "git/cargo" -> rc bun -> "git/cargo/bun"
        let res = compute_next_stack(Some("git/cargo"), Some("bun"));
        assert_eq!(res, "git/cargo/bun");
    }

    #[test]
    fn test_stack_deep_empty_nesting() {
        // L2: "/" -> rc -> "//"
        let res = compute_next_stack(Some("/"), None);
        assert_eq!(res, "//");
    }

    // --- resolve_program のテスト (既存) ---

    #[test]
    #[cfg(target_os = "windows")]
    fn test_resolve_program_windows_cmd() {
        let res = resolve_program("cmd");
        assert!(res.to_lowercase().ends_with(".exe"));
        assert_ne!(res, "cmd");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_resolve_program_windows_not_found() {
        let cmd = "non_existent_command_12345aaaaaaaa";
        let res = resolve_program(cmd);
        assert_eq!(res, cmd);
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_resolve_program_unix_noop() {
        let cmd = "ls";
        let res = resolve_program(cmd);
        assert_eq!(res, cmd);
    }
}
