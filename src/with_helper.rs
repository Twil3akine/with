use rustyline::{Completer, Helper, Hinter, Validator, highlight::Highlighter};
use std::borrow::Cow;

// --- Rustylineのヘルパー設定 ---
#[derive(Helper, Completer, Hinter, Validator)]
pub struct WithHelper {}

// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_CYAN: &str = "\x1b[36m";
const COLOR_MAGENTA: &str = "\x1b[35m"; // ★ 追加: ブランチ用
const STYLE_BOLD: &str = "\x1b[1m";
const STYLE_RESET: &str = "\x1b[0m";

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
