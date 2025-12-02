use rustyline::{
    Context, Helper, Hinter, Validator,
    completion::{Completer, FilenameCompleter, Pair},
    highlight::Highlighter,
};
use std::{
    borrow::Cow,
    iter::{IntoIterator, Iterator},
    option::Option::{self, Some},
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
const COLOR_CYAN: &str = "\x1b[36m";
const COLOR_MAGENTA: &str = "\x1b[35m";
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
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        if let Some(end_arrow) = prompt.rfind("> ") {
            if prompt.starts_with('(') {
                if let Some(close_paren) = prompt.find(") ") {
                    // カッコの中身全体を取得 (例: "src : main" または "src")
                    let content_inside = &prompt[1..close_paren];

                    let styled_content = if let Some(sep_idx) = content_inside.find(": ") {
                        // "src : main" のように区切りがある場合
                        let path_part = &content_inside[0..sep_idx];
                        let branch_part = &content_inside[sep_idx + 2..]; // ": " は2文字

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
                        // "src" だけの場合
                        format!("{}{}{}", COLOR_GREEN, content_inside, STYLE_RESET)
                    };

                    // コマンド部分の取得 logic
                    let cmd_start = close_paren + 2;
                    let cmd_part = &prompt[cmd_start..end_arrow];

                    let styled = format!(
                        "{}({}) {}{}{}{}{}{}> ",
                        STYLE_BOLD,     // (
                        styled_content, // 中身（色付き済み）
                        STYLE_BOLD,     // )
                        COLOR_CYAN,     // cmd color
                        cmd_part,
                        STYLE_RESET, // cmd color reset
                        STYLE_BOLD,  // >
                        STYLE_RESET  // reset all
                    );
                    return Cow::Owned(styled);
                }
            }
            // ... (パターン2: ディレクトリ表示なしの場合は既存のまま) ...
            else {
                let cmd_part = &prompt[0..end_arrow];
                let styled = format!(
                    "{}{}{}{}{}{}> ",
                    STYLE_BOLD, COLOR_CYAN, cmd_part, STYLE_RESET, STYLE_BOLD, STYLE_RESET
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
            "status", "commit", "add", "push", "pull", "fetch", "checkout", "branch", "diff",
            "log", "merge", "rebase", "reset", "switch", "reflog",
        ],
        "cargo" => vec![
            "build", "check", "clean", "doc", "new", "init", "run", "test", "bench", "update",
            "search", "publish", "install",
        ],
        "pnpm" | "bun" | "npm" => vec![
            "install", "start", "test", "run", "build", "publish", "ci", "audit",
        ],
        "docker" => vec![
            "ps", "run", "exec", "build", "pull", "push", "images", "network", "volume", "compose",
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
}
