use rustyline::{
    Context, Helper, Hinter, Validator,
    completion::{Completer, FilenameCompleter, Pair},
    highlight::Highlighter,
};
use std::{
    borrow::Cow,
    iter::{IntoIterator, Iterator},
    option::Option::{self, None, Some},
    vec::Vec,
};

// --- Rustylineのヘルパー設定 ---
#[derive(Helper, Hinter, Validator)]
pub struct WithHelper {
    pub completer: FilenameCompleter,
    pub context_program: Option<String>,
}

// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_MAGENTA: &str = "\x1b[35m";
const COLOR_CYAN: &str = "\x1b[36m";
const COLOR_WHITE: &str = "\x1b[37m";
const STYLE_BOLD: &str = "\x1b[1m";
const STYLE_RESET: &str = "\x1b[0m";

impl Completer for WithHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // 処理を標準のFilenameCompleterに丸投げ（委譲）する
        let line_up_to_cursor = &line[..pos];

        let args = shell_words::split(line_up_to_cursor).unwrap_or_default();

        let has_trailing_space = line_up_to_cursor
            .chars()
            .last()
            .is_some_and(|c| c.is_whitespace());
        let current_arg_index = if args.is_empty() {
            0
        } else if has_trailing_space {
            args.len()
        } else {
            args.len() - 1
        };

        let target_cmd = if let Some(prog) = &self.context_program {
            if current_arg_index == 0 {
                Some(prog.as_str())
            } else {
                None
            }
        } else if current_arg_index == 1 && !args.is_empty() {
            Some(args[0].as_str())
        } else {
            None
        };

        if let Some(cmd) = target_cmd {
            let word = if has_trailing_space {
                ""
            } else {
                args.last().map(|s| s.as_str()).unwrap_or("")
            };

            let start = pos - word.len();

            let candidates = get_subcommands(cmd);
            let matches: Vec<Pair> = candidates
                .into_iter()
                .filter(|c| c.starts_with(word))
                .map(|c| Pair {
                    display: c.to_string(),
                    replacement: c.to_string(),
                })
                .collect();

            if !matches.is_empty() {
                return Ok((start, matches));
            }
        }

        self.completer.complete(line, pos, ctx)
    }
}

impl Highlighter for WithHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // 色付けする必要がない（空行など）場合はそのまま返す
        if line.trim().is_empty() {
            return Cow::Borrowed(line);
        }

        // 単語の境界（開始位置と終了位置）を探す簡易パーサ
        // ※ shell_words::split だと空白が消えてしまうため、表示用に位置だけ特定する
        let mut word_ranges = Vec::new();
        let mut in_word = false;
        let mut start_idx = 0;
        let mut in_quote = None; // クォート内判定用

        for (i, c) in line.char_indices() {
            if let Some(q) = in_quote {
                if c == q {
                    in_quote = None; // クォート終了
                }
            } else if c == '"' || c == '\'' {
                in_quote = Some(c); // クォート開始
                if !in_word {
                    start_idx = i;
                    in_word = true;
                }
            } else if c.is_whitespace() {
                if in_word {
                    word_ranges.push((start_idx, i)); // 単語の終わり
                    in_word = false;
                }
            } else if !in_word {
                start_idx = i; // 単語の始まり
                in_word = true;
            }
        }
        // 最後の単語を処理
        if in_word {
            word_ranges.push((start_idx, line.len()));
        }

        // --- 色判定 ---
        // 親コマンド名の特定
        let parent_cmd_name = if let Some(ctx_prog) = &self.context_program {
            Some(ctx_prog.as_str())
        } else if !word_ranges.is_empty() {
            let (s, e) = word_ranges[0];
            Some(&line[s..e])
        } else {
            None
        };

        // 親コマンドがサブコマンドを持つコマンドかを確認
        let expects_subcommand = parent_cmd_name
            .map(|name| !get_subcommands(name).is_empty())
            .unwrap_or(false);

        // 何番目の単語をどう色付けするか決める
        let (prog_idx, subcmd_idx) = if self.context_program.is_some() {
            // Case A: `with git` (コンテキストあり)
            // 0番目の単語 = サブコマンド (例: "status")
            (None, if expects_subcommand { Some(0) } else { None })
        } else {
            // Case B: `with` 単体 (コンテキストなし)
            // 0番目の単語 = 親コマンド (例: "git")
            // 1番目の単語 = サブコマンド (例: "status")
            (Some(0), if expects_subcommand { Some(1) } else { None })
        };

        // 文字列を再構築する
        let mut new_line = String::with_capacity(line.len() + 20);
        let mut last_idx = 0;

        for (i, (start, end)) in word_ranges.iter().enumerate() {
            // 前の単語との間の空白などを追加
            new_line.push_str(&line[last_idx..*start]);

            let word = &line[*start..*end];

            // 色を決定
            if Some(i) == prog_idx {
                // 親コマンド: 緑
                new_line.push_str(COLOR_CYAN);
                new_line.push_str(word);
                new_line.push_str(STYLE_RESET);
            } else if Some(i) == subcmd_idx {
                // サブコマンド: シアン
                new_line.push_str(COLOR_GREEN);
                new_line.push_str(word);
                new_line.push_str(STYLE_RESET);
            } else if word.starts_with('"') || word.starts_with('\'') {
                new_line.push_str(COLOR_WHITE);
                new_line.push_str(word);
                new_line.push_str(STYLE_RESET);
            } else if word.starts_with('-') {
                // オプション引数: 黄色
                new_line.push_str(COLOR_YELLOW);
                new_line.push_str(word);
                new_line.push_str(STYLE_RESET);
            } else {
                // その他: そのまま
                new_line.push_str(word);
            }

            last_idx = *end;
        }

        // 末尾の残りの文字（空白など）を追加
        new_line.push_str(&line[last_idx..]);

        Cow::Owned(new_line)
    }

    fn highlight_char(
        &self,
        _line: &str,
        _pos: usize,
        _kind: rustyline::highlight::CmdKind,
    ) -> bool {
        true
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if let Some(end_arrow) = prompt.rfind("> ") {
            // パターン1: ディレクトリ情報あり "(.: branch) git/cargo >"
            if prompt.starts_with('(') {
                if let Some(close_paren) = prompt.find(") ") {
                    // --- ディレクトリ表示部分 (既存のまま) ---
                    let content_inside = &prompt[1..close_paren];

                    let styled_content = if let Some(sep_idx) = content_inside.find(": ") {
                        let path_part = &content_inside[0..sep_idx];
                        let branch_part = &content_inside[sep_idx + 2..];

                        format!(
                            "{}{}{}{}{}: {}{}{}",
                            COLOR_GREEN, // Path Color
                            path_part,
                            STYLE_RESET,
                            STYLE_BOLD, // Separator Style
                            STYLE_RESET,
                            COLOR_MAGENTA, // Branch Color
                            branch_part,
                            STYLE_RESET // Reset before ')'
                        )
                    } else {
                        format!("{}{}{}", COLOR_GREEN, content_inside, STYLE_RESET)
                    };
                    // -------------------------------------

                    let cmd_start = close_paren + 2;
                    let cmd_part = &prompt[cmd_start..end_arrow];

                    // 最後のスラッシュで分割
                    // 例: "//git" -> parent="/", current="git" -> path_str="//", current_str="git"
                    let (path_str, current_str) =
                        if let Some((parent, current)) = cmd_part.rsplit_once('/') {
                            (format!("{}/", parent), current)
                        } else {
                            (String::new(), cmd_part)
                        };

                    // ★修正済み: フォーマット指定子 {} の数を引数に合わせています
                    let styled = format!(
                        "{}({}) {}{}{}{}{}{}{}{}> {}", // {} を1つ追加 (合計11個)
                        STYLE_BOLD,                    // 1: (
                        styled_content,                // 2: dir info
                        STYLE_BOLD,                    // 3: )
                        STYLE_RESET,                   // ★4: 追加！ ここで一度太字をリセットします
                        COLOR_CYAN,                    // 5: path color
                        path_str,                      // 6: path string (これで細字+水色になります)
                        STYLE_BOLD,                    // 7: current bold
                        current_str,                   // 8: current string (これは太字+水色)
                        STYLE_RESET,                   // 9: reset current
                        STYLE_BOLD,                    // 10: >
                        STYLE_RESET                    // 11: reset all
                    );
                    return Cow::Owned(styled);
                }
            }
            // パターン2: ディレクトリ情報なし "git/cargo >"
            else {
                let cmd_part = &prompt[0..end_arrow];

                let (path_str, current_str) =
                    if let Some((parent, current)) = cmd_part.rsplit_once('/') {
                        (format!("{}/", parent), current)
                    } else {
                        (String::new(), cmd_part)
                    };

                // ★修正済み: こちらも {} の数を修正
                let styled = format!(
                    "{}{}{}{}{}{}{}{}> {}",
                    STYLE_BOLD,  // 1: Bold start
                    STYLE_RESET, // 2: Reset (safety)
                    COLOR_CYAN,  // 3: Color
                    path_str,    // 4: path (Fine)
                    STYLE_BOLD,  // 5: current Bold
                    current_str, // 6: current
                    STYLE_RESET, // 7: reset
                    STYLE_BOLD,  // 8: >
                    STYLE_RESET  // 9: reset all
                );
                return Cow::Owned(styled);
            }
        }
        Cow::Borrowed(prompt)
    }
}

/// 指定されたコマンドに対するサブコマンドのリストを返す
pub fn get_subcommands(command: &str) -> Vec<&str> {
    match command {
        "git" => vec![
            "add", "bisect", "blame", "branch", "checkout", "clean", "clone", "commit", "config",
            "diff", "fetch", "grep", "init", "log", "merge", "mv", "pull", "push", "rebase",
            "reflog", "remote", "reset", "restore", "revert", "rm", "show", "stash", "status",
            "switch", "tag",
        ],

        "cargo" => vec![
            "add", "bench", "build", "check", "clean", "clippy", "doc", "expand", "fix", "fmt",
            "init", "install", "metadata", "new", "publish", "remove", "run", "search", "test",
            "tree", "update", "yank",
        ],

        "pnpm" | "bun" | "npm" | "yarn" => vec![
            "add",
            "audit",
            "build",
            "ci",
            "create",
            "exec",
            "init",
            "install",
            "link",
            "list",
            "outdated",
            "pack",
            "publish",
            "remove",
            "restart",
            "run",
            "start",
            "stop",
            "test",
            "uninstall",
            "unlink",
            "update",
            "why",
        ],

        "docker" => vec![
            "attach", "build", "compose", "cp", "create", "diff", "events", "exec", "export",
            "history", "images", "import", "info", "inspect", "kill", "load", "login", "logout",
            "logs", "network", "pause", "port", "ps", "pull", "push", "rename", "restart", "rm",
            "rmi", "run", "save", "search", "start", "stats", "stop", "system", "tag", "top",
            "unpause", "update", "version", "volume", "wait",
        ],

        "uv" => vec![
            "add", "cache", "clean", "export", "init", "lock", "pip", "python", "remove", "run",
            "self", "sync", "tool", "tree", "venv", "version",
        ],

        "pip" | "pip3" => vec![
            "check",
            "config",
            "debug",
            "download",
            "freeze",
            "hash",
            "install",
            "list",
            "show",
            "uninstall",
            "wheel",
        ],

        "kubectl" | "k" => vec![
            "apply",
            "api-resources",
            "attach",
            "auth",
            "autoscale",
            "certificate",
            "cluster-info",
            "config",
            "cordon",
            "cp",
            "create",
            "delete",
            "describe",
            "diff",
            "drain",
            "edit",
            "exec",
            "explain",
            "expose",
            "get",
            "label",
            "logs",
            "options",
            "patch",
            "plugin",
            "port-forward",
            "proxy",
            "replace",
            "rollout",
            "run",
            "scale",
            "set",
            "taint",
            "top",
            "uncordon",
            "version",
            "wait",
        ],

        "terraform" | "tf" => vec![
            "apply",
            "console",
            "destroy",
            "fmt",
            "get",
            "graph",
            "import",
            "init",
            "login",
            "logout",
            "output",
            "plan",
            "providers",
            "refresh",
            "show",
            "state",
            "taint",
            "test",
            "untaint",
            "validate",
            "version",
            "workspace",
        ],

        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::Context;
    use rustyline::history::DefaultHistory;

    // テスト用のヘルパー作成関数
    fn create_helper(context_program: Option<&str>) -> WithHelper {
        WithHelper {
            completer: FilenameCompleter::new(),
            context_program: context_program.map(|s| s.to_string()),
        }
    }

    // 補完結果に特定の文字列が含まれているかチェックする
    fn assert_contains(candidates: &[Pair], value: &str) {
        let found = candidates.iter().any(|p| p.replacement == value);
        let debug_list: Vec<&str> = candidates.iter().map(|p| p.replacement.as_str()).collect();
        assert!(
            found,
            "Expected completion '{}' not found in {:?}",
            value, debug_list
        );
    }

    // 補完結果に特定の文字列が含まれて *いない* ことチェックする
    fn assert_not_contains(candidates: &[Pair], value: &str) {
        let found = candidates.iter().any(|p| p.replacement == value);
        assert!(!found, "Unexpected completion '{}' found", value);
    }

    #[test]
    fn test_context_mode_subcommand() {
        // ケース: `with git` で起動中 (context_program = "git")
        let helper = create_helper(Some("git"));
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);

        // 1. "st" と入力 -> "status" が出るべき (index 0)
        let line = "st";
        let pos = line.len();
        let (start, res) = helper.complete(line, pos, &ctx).unwrap();

        // start位置は 0 (先頭から置換)
        assert_eq!(start, 0);
        assert_contains(&res, "status");

        // 2. "status " (スペースあり) -> サブコマンド補完は出ないべき (index 1)
        // ※実際にはファイル補完が走るが、ここでは "status" 等が出ないことを確認
        let line = "status ";
        let pos = line.len();
        let (_, res) = helper.complete(line, pos, &ctx).unwrap();

        // 次の引数には "status" コマンドは提案されないはず
        assert_not_contains(&res, "status");
    }

    #[test]
    fn test_no_context_mode_subcommand() {
        // ケース: `with` 単体起動 (context_program = None)
        let helper = create_helper(None);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);

        // 1. "git " (スペース直後) -> "status" 等が出るべき (index 1)
        let line = "git ";
        let pos = line.len();
        let (start, res) = helper.complete(line, pos, &ctx).unwrap();

        // start位置は pos と同じ (現在のカーソル位置から挿入)
        assert_eq!(start, pos);
        assert_contains(&res, "status");
        assert_contains(&res, "commit");

        // 2. "git st" -> "status" が出るべき
        let line = "git st";
        let pos = line.len();
        let (start, res) = helper.complete(line, pos, &ctx).unwrap();

        // "git " の長さは4なので、4バイト目から置換開始
        assert_eq!(start, 4);
        assert_contains(&res, "status");

        // 3. "cargo b" -> "build", "bench" が出るべき
        let line = "cargo b";
        let pos = line.len();
        let (_, res) = helper.complete(line, pos, &ctx).unwrap();

        assert_contains(&res, "build");
        assert_contains(&res, "bench");
        // cargoの補完リストに "status" はないはず
        assert_not_contains(&res, "status");
    }

    #[test]
    fn test_ignore_other_args() {
        // ケース: 第3引数以降は反応しない
        let helper = create_helper(None);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);

        // "git commit -m" -> ここで "status" とか出ると困る
        let line = "git commit -m";
        let pos = line.len();
        let (_, res) = helper.complete(line, pos, &ctx).unwrap();

        assert_not_contains(&res, "status");
        assert_not_contains(&res, "commit");
    }

    #[test]
    fn test_unknown_command() {
        // 未登録のコマンド
        let helper = create_helper(None);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);

        let line = "unknown_cmd ";
        let pos = line.len();
        let (_, res) = helper.complete(line, pos, &ctx).unwrap();

        // 何もカスタム補完されない（ファイル補完に落ちる）
        assert_not_contains(&res, "status");
    }

    // --- ハイライト（色付け）のテスト ---

    #[test]
    fn test_highlight_registered_subcommand() {
        // ケース: git status (登録済みコマンド + サブコマンド)
        let helper = create_helper(None);
        let line = "git status";
        let highlighted = helper.highlight(line, 0);

        // 親コマンド(git, $1)は CYAN
        assert!(
            highlighted.contains(COLOR_CYAN),
            "Parent cmd should be cyan"
        );
        assert!(highlighted.contains("git"));

        // サブコマンド(status, $2)は GREEN
        assert!(
            highlighted.contains(COLOR_GREEN),
            "Subcommand should be green"
        );
        assert!(highlighted.contains("status"));
    }

    #[test]
    fn test_highlight_normal_argument() {
        // ケース: mkdir my_folder (登録なしコマンド + 通常引数)
        // mkdir は get_subcommands に登録されていない想定
        let helper = create_helper(None);
        let line = "mkdir my_folder";
        let highlighted = helper.highlight(line, 0);

        // 親コマンド(mkdir, $1)は CYAN
        assert!(highlighted.contains(COLOR_CYAN));
        assert!(highlighted.contains("mkdir"));

        // 引数(my_folder)はデフォルト色のまま (GREENが含まれていないこと)
        assert!(!highlighted.contains(&format!("{}{}", COLOR_GREEN, "my_folder")));
    }

    #[test]
    fn test_highlight_flags() {
        // ケース: -v や --help (フラグ)
        let helper = create_helper(None);
        let line = "ls -v --help";
        let highlighted = helper.highlight(line, 0);

        // ls は CYAN
        assert!(highlighted.contains("ls"));

        // -v, --help は YELLOW
        assert!(highlighted.contains(COLOR_YELLOW));
        assert!(highlighted.contains("-v"));
        assert!(highlighted.contains("--help"));
    }

    #[test]
    fn test_highlight_quotes() {
        // ケース: echo "hello world" (文字列リテラル)
        let helper = create_helper(None);
        let line = "echo \"hello world\"";
        let highlighted = helper.highlight(line, 0);

        // "hello world" は WHITE
        assert!(
            highlighted.contains(COLOR_WHITE),
            "Quoted string should be white"
        );
        assert!(highlighted.contains("\"hello world\""));

        // echo は CYAN
        assert!(highlighted.contains(COLOR_CYAN));
    }

    #[test]
    fn test_highlight_context_mode() {
        // ケース: with git 起動中に "status" と入力
        let helper = create_helper(Some("git"));
        let line = "status";
        let highlighted = helper.highlight(line, 0);

        // contextがあるので、0単語目("status")はサブコマンド($2)扱い -> GREEN
        assert!(highlighted.contains(COLOR_GREEN));
        assert!(highlighted.contains("status"));

        // 親コマンド($1)の色(CYAN)は使われないはず
        assert!(!highlighted.contains(COLOR_CYAN));
    }
}
