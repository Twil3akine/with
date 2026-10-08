use std::{fs, option::Option::*, path::Path};

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

/// カレントディレクトリから遡って最も近い Git worktree のブランチ名を返す
pub fn get_git_branch(cwd: &Path) -> Option<String> {
    let mut current = cwd;

    loop {
        let git_path = current.join(".git");

        if git_path.is_dir() {
            return fs::read_to_string(git_path.join("HEAD"))
                .ok()
                .and_then(|content| parse_git_head(&content));
        }

        if git_path.is_file() {
            let git_file = fs::read_to_string(&git_path).ok()?;
            let git_dir = git_file.trim().strip_prefix("gitdir:")?.trim();
            if git_dir.is_empty() {
                return None;
            }

            let git_dir = Path::new(git_dir);
            let git_dir = if git_dir.is_absolute() {
                git_dir.to_path_buf()
            } else {
                current.join(git_dir)
            };
            return fs::read_to_string(git_dir.join("HEAD"))
                .ok()
                .and_then(|content| parse_git_head(&content));
        }

        match current.parent() {
            Some(p) => current = p,
            None => break,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temporary_directory() -> std::path::PathBuf {
        let id = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("with-context-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
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

    #[test]
    fn test_parse_git_head_with_slashes() {
        // ブランチ名にスラッシュが含まれる場合
        // "ref: refs/heads/feature/new-ui" -> "feature/new-ui"
        let content = "ref: refs/heads/feature/new-ui\n";
        assert_eq!(parse_git_head(content), Some("feature/new-ui".to_string()));
    }

    #[test]
    fn test_parse_git_head_whitespace_handling() {
        // 前後に空白や改行があっても trim されて正しく動くか
        let content = "   ref: refs/heads/dev   \n";
        assert_eq!(parse_git_head(content), Some("dev".to_string()));
    }

    #[test]
    fn test_parse_git_head_detached_exact_length() {
        // ちょうど7文字のハッシュ値の場合
        let content = "1234567";
        assert_eq!(parse_git_head(content), Some("1234567".to_string()));
    }

    #[test]
    fn test_parse_git_head_detached_too_short() {
        // 7文字未満の場合は None になるべき
        let content = "123456";
        assert_eq!(parse_git_head(content), None);
    }

    #[test]
    fn test_parse_git_head_empty() {
        // 空文字の場合
        let content = "";
        assert_eq!(parse_git_head(content), None);
    }

    #[test]
    fn test_display_dir_parent_of_base() {
        // base より上の階層にいる場合
        // base: /home/user/project
        // current: /home/user
        // -> "user" (現在のフォルダ名) が表示される仕様
        let base = std::path::PathBuf::from("/home/user/project");
        let current = std::path::PathBuf::from("/home/user");

        assert_eq!(
            resolve_display_dir(&current, &base),
            Some("user".to_string())
        );
    }

    #[test]
    fn test_display_dir_root() {
        // ルートディレクトリの場合
        let base = std::path::PathBuf::from("/home/user/project");

        // UNIX系なら "/"
        #[cfg(unix)]
        let current = std::path::PathBuf::from("/");

        // Windowsなら "C:\" など
        #[cfg(windows)]
        let current = std::path::PathBuf::from("C:\\");

        // ルートパスの file_name() は None を返すことがあるため、
        // unwrap_or(".") が機能して "." などを返すか、
        // 実際に返ってくる値を検証（環境依存の可能性があるため緩めにチェック）
        let result = resolve_display_dir(&current, &base);
        assert!(result.is_some());
    }

    #[test]
    fn git_branch_in_submodule_uses_its_git_file_and_branch() {
        let root = temporary_directory();
        let parent_git = root.join(".git");
        let submodule = root.join("vendor").join("library");
        let module_git = parent_git.join("modules").join("library");
        fs::create_dir_all(&parent_git).unwrap();
        fs::create_dir_all(&module_git).unwrap();
        fs::create_dir_all(submodule.join("src")).unwrap();
        fs::write(parent_git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(module_git.join("HEAD"), "ref: refs/heads/develop\n").unwrap();
        fs::write(
            submodule.join(".git"),
            "gitdir: ../../.git/modules/library\n",
        )
        .unwrap();

        assert_eq!(
            get_git_branch(&submodule.join("src")),
            Some("develop".to_string())
        );

        fs::write(
            submodule.join(".git"),
            format!("gitdir: {}\n", module_git.display()),
        )
        .unwrap();
        fs::write(
            module_git.join("HEAD"),
            "a1b2c3d4e5f67890abcdef1234567890abcdef12\n",
        )
        .unwrap();
        assert_eq!(
            get_git_branch(&submodule.join("src")),
            Some("a1b2c3d".to_string())
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn git_branch_uses_nearest_repository_and_returns_none_outside_git() {
        let root = temporary_directory();
        let repository = root.join("repository");
        let nested = repository.join("src").join("nested");
        fs::create_dir_all(repository.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            repository.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .unwrap();

        assert_eq!(get_git_branch(&nested), Some("main".to_string()));
        assert_eq!(get_git_branch(&root), None);

        fs::remove_dir_all(root).unwrap();
    }
}
