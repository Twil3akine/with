use rustyline::{Completer, Helper, Hinter, Validator, highlight::Highlighter};
use std::borrow::Cow;

// --- Rustylineのヘルパー設定 ---
#[derive(Helper, Completer, Hinter, Validator)]
pub struct WithHelper {}

// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_CYAN: &str = "\x1b[36m";
const STYLE_BOLD: &str = "\x1b[1m";
const STYLE_RESET: &str = "\x1b[0m";

// プロンプトの色付けロジックを実装
impl Highlighter for WithHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        // プロンプトの末尾 "> " を探す
        if let Some(end_arrow) = prompt.rfind("> ") {
            // パターン1: "(dir) cmd> " の場合（先頭が '(' で始まる）
            if prompt.starts_with('(') {
                if let Some(close_paren) = prompt.find(") ") {
                    // (dir) 部分
                    let dir_part = &prompt[1..close_paren];
                    // cmd 部分 ( ") " の後ろから "> " の前まで)
                    // start: close_paren + 2 (つまり ") "の長さ)
                    let cmd_start = close_paren + 2;
                    let cmd_part = &prompt[cmd_start..end_arrow];

                    let styled = format!(
                        "{}({}{}{}) {}{}{}{}{}{}> ",
                        STYLE_BOLD,  // (
                        COLOR_GREEN, // dir color
                        dir_part,
                        STYLE_RESET, // dir color reset
                        STYLE_BOLD,  // )
                        COLOR_CYAN,  // cmd color
                        cmd_part,
                        STYLE_RESET, // cmd color reset
                        STYLE_BOLD,  // >
                        STYLE_RESET  // reset all
                    );
                    return Cow::Owned(styled);
                }
            }
            // パターン2: "cmd> " の場合 (ディレクトリ表示なし)
            else {
                let cmd_part = &prompt[0..end_arrow];
                let styled = format!(
                    "{}{}{}{}{}{}> ",
                    STYLE_BOLD, // cmd style start
                    COLOR_CYAN, // cmd color
                    cmd_part,
                    STYLE_RESET, // cmd color reset
                    STYLE_BOLD,  // >
                    STYLE_RESET  // reset all
                );
                return Cow::Owned(styled);
            }
        }

        // パースできなかったらそのまま返す
        Cow::Borrowed(prompt)
    }
}
