use rustyline::{Completer, Helper, Hinter, Validator, highlight::Highlighter};
use std::borrow::Cow;

// --- Rustylineのヘルパー設定 ---
#[derive(Helper, Completer, Hinter, Validator)]
pub struct WithHelper {}

// プロンプトの色付け用
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_CYAN: &str = "\x1b[36m";
const STYLE_BOLD: &str = "\x1b[1m";
const STYLE_RESET: &str = "\x1b[0m"; // 色も太字も全部リセット

// プロンプトの装飾用マーカー（Highlighterでの検知にも使用）
const PROMPT_OPEN: &str = " [";
const PROMPT_CLOSE: &str = "]> ";

// プロンプトの色付けロジックを実装
impl Highlighter for WithHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        // プロンプトが "cmd (dir)> " の形かチェックして色付け
        // "(" と ")> " で分割して場所を特定します
        if let (Some(start), Some(end)) = (prompt.find(PROMPT_OPEN), prompt.find(PROMPT_CLOSE)) {
            let cmd_part = &prompt[0..start];
            // PROMPT_OPENの長さ分ずらす
            let dir_part = &prompt[start + PROMPT_OPEN.len()..end];

            let styled = format!(
                "{}{}{}{} [{}{}{}]{}{}> ",
                STYLE_BOLD,
                COLOR_CYAN,
                cmd_part,
                STYLE_RESET, // Cmd
                COLOR_GREEN,
                dir_part,
                STYLE_RESET, // Dir
                STYLE_BOLD,  // Arrow
                STYLE_RESET
            );
            return Cow::Owned(styled);
        }

        // パースできなかったらそのまま返す
        Cow::Borrowed(prompt)
    }
}
