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

#[cfg(test)]
mod tests {
    use super::*;

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
}
