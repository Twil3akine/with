# v1.0 readiness audit

**確認日:** 2026-10-08 (UTC)  
**対象:** GitHub default branch `master` の snapshot `c87743f52d33d112f0444f8e2a65702a2256d7bc`。GitHub API の open issue / release metadata と、同 SHA のソース・README・workflow を照合した。Issue 本文の最終更新日は古い記録を含む。以下の状態は監査 snapshot に対する判定で、別workerの作業は取り込んでいない。

## v1.0 前に確認・解消が必要

- **#17 クロス OS 検証:** issue は open。CI は Ubuntu のみ、push trigger は `main`/`dev`/`actions` で default branch `master` を含まない。release workflow では Windows / x86_64 macOS もタグ時にテストするが、PR 毎の実行ではない。Windows/macOS/Linux のパス、CRLF/LF、Ctrl-C/Ctrl-D、`PATH`/`USER` の確認記録は見当たらない。Windows では `shell-words` の前に `\\` をすべて `/` へ置換するが、該当するクロス OS テストもない。これは未検証リスクであり、実機不具合とは断定しない。[#17](https://github.com/Twil3akine/with/issues/17) · [CI](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/.github/workflows/ci.yml#L3-L17) · [Windows parser](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/parser.rs#L23-L45)
- **#18 README / デモ:** インストール手順と基本操作例は README にある。README の Ctrl-C 説明は実装と矛盾し、`ReadlineError::Interrupted` は EOF と同じく REPL を終了する。また README は `rc`/`recursive` の再帰起動を説明しておらず、`exit`/`e` は親を含めて終了 (`process::exit(127)`)、`quit`/`q` は現在の REPL のみ終了する差も書かれていない。さらに License は Google 検索 URL で、同梱 MIT license へのリンクではない。これらの正確さはリリース前に整える。[#18](https://github.com/Twil3akine/with/issues/18) · [README](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/README.md#L17-L20) · [README internal commands / License](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/README.md#L106-L127) · [parser exit / rc](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/parser.rs#L35-L85) · [main exit handling](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/main.rs#L150-L162)
- **ポートフォリオ用途では、実動デモを v1.0 の提示物に含めるべき。** Issue #18 が求める GIF は現 README にない。導入・使用説明だけでは、主機能である対話入力、補完、コンテキスト維持、実コマンド実行が動く証拠を読者が確認できないため、「デモは不要」とは言い切れない。最低限、現 snapshot で動く代表的な一連の操作を短い実録 GIF または同等の動画で示す。Issue #14 の設定ファイル alias 機能は未実装なので、デモでその機能があるように見せない。CI badge は状態表示であり、実動作の証拠にはならないため v1.0 後でもよい。[#18](https://github.com/Twil3akine/with/issues/18) · [README](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/README.md)
- **Windows 向け `clear`/`cls` と `pwd`:** どちらも内部で OS shell builtin を呼ぶのではなく `Command::new` で外部プログラムを起動する。`cls` も `clear` にマップされるため、README の全 OS 案内との一致は Windows 上で未確認。これは README の主張と実動作を確認してリリース前に判定する。[README](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/README.md#L106-L115) · [main.rs](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/main.rs#L135-L142) · [parser.rs](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/parser.rs#L78-L85)
- **#37 / #42 認証・push 報告:** どちらも open。Issue の報告だけでは再現条件と原因を確定できない。snapshot の実行経路は引数を `Command::new` に渡して子プロセスを待つだけで、認証情報処理の実装は見当たらない。Git push は README が例示する中心的ワークフローなので、v1.0 前に報告が再現するか、`with` が原因かを確認して扱いを明確にする。現時点で「修正済み」とは判定しない。[#37](https://github.com/Twil3akine/with/issues/37) · [#42](https://github.com/Twil3akine/with/issues/42) · [executor](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/executor.rs#L48-L87)
- **#41 submodule の branch 表示:** open。コードは `.git/HEAD` を探すため、submodule の `.git` がファイルの場合に親側の branch を表示する報告と整合する。プロンプト上の Git 情報の正確さに関わるため、v1.0 前に扱いを確認する。#38 と #41 は別workerが修正中との連絡を受けているが、監査対象 SHA `c87743f` には含めておらず、修正済み判定にも含めない。[#41](https://github.com/Twil3akine/with/issues/41) · [context.rs](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/src/context.rs#L40-L58)
- **リリースの識別整合:** `Cargo.toml` は `0.2.0`、確認できた最新 GitHub Release は `v0.4.0`（2025-12-03 公開）。v1.0 のタグ、配布物、crate metadata の対応を出荷前にそろえる。[Cargo.toml](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/Cargo.toml#L1-L8) · [Releases](https://github.com/Twil3akine/with/releases)
- **Apple Silicon 配布:** `v0.4.0` assets に macOS arm64 はなく、workflow の macOS target は `x86_64-apple-darwin` のみ。macOS 対応を案内する v1.0 の配布範囲として、Apple Silicon での利用可否を明記・確認する。[Release workflow](https://github.com/Twil3akine/with/blob/c87743f52d33d112f0444f8e2a65702a2256d7bc/.github/workflows/release.yml#L16-L45) · [v0.4.0](https://github.com/Twil3akine/with/releases/tag/v0.4.0)

## Open issue 全13件の snapshot 判定

GitHub API で全件 open と確認。実装判定は `c87743f` のみを対象とする。

| Issue | snapshot 上の実装状況 | v1.0 の扱い |
| --- | --- | --- |
| [#14 設定・alias](https://github.com/Twil3akine/with/issues/14) | 設定読込・アプリ内 alias なし。README の shell alias は `with` 起動用で別物。 | v1.0 後。機能追加 issue。 |
| [#15 シンタックスハイライト](https://github.com/Twil3akine/with/issues/15) | `Highlighter` と色付けはあるが、実行可能性検査や不正コマンドの赤表示なし。部分実装。 | 現行の色分けはあるため、完全な valid/invalid 判定は v1.0 後。Issue は未完了のまま扱う。 |
| [#16 context push/pop](https://github.com/Twil3akine/with/issues/16) | `rc` による再帰 context stack 表示はあるが、対話中の `+`/`-` push/pop 操作なし。部分実装。 | v1.0 後。新しい対話操作。 |
| [#17 cross-platform 検証](https://github.com/Twil3akine/with/issues/17) | 実装の一部に OS 分岐はあるが、要求 OS での検証記録なし。 | v1.0 前。 |
| [#18 README / demo GIF](https://github.com/Twil3akine/with/issues/18) | 導入と基本例はある。実録デモ、badge なし。 | README の差異修正とポートフォリオ用デモは v1.0 前。badge は v1.0 後。 |
| [#25 TUI 選択](https://github.com/Twil3akine/with/issues/25) | なし。 | v1.0 後。新機能 issue。 |
| [#26 長時間コマンド通知](https://github.com/Twil3akine/with/issues/26) | なし。 | v1.0 後。新機能 issue。 |
| [#34 ローカル補完定義](https://github.com/Twil3akine/with/issues/34) | 外部定義ファイルの読込なし。 | v1.0 後。新機能 issue。 |
| [#37 push 時の認証再要求](https://github.com/Twil3akine/with/issues/37) | 原因・再現条件を snapshot だけでは確定できず、認証対応コードなし。 | v1.0 前に再現性と扱いを判定。修正済みとは数えない。 |
| [#38 重複 prefix](https://github.com/Twil3akine/with/issues/38) | `git` context で `git push` を再度入力した場合の prefix 除去なし。 | v1.0 後の利便性 issue。別worker修正中との連絡はあるが、snapshot 判定は未実装。 |
| [#39 context 切替](https://github.com/Twil3akine/with/issues/39) | 内部 `switch` なし。 | v1.0 後。新しい対話操作。 |
| [#41 submodule branch 表示](https://github.com/Twil3akine/with/issues/41) | `.git` ファイルを解決する処理なし。issue の表示誤りを修正していない。 | v1.0 前に結果確認。別worker修正中との連絡はあるが、snapshot 判定は未実装。 |
| [#42 push 認証問題](https://github.com/Twil3akine/with/issues/42) | 報告が未詳細で原因不明。認証対応コードなし。 | v1.0 前に再現性と扱いを判定。修正済みとは数えない。 |

この13件から、受け入れ条件まで完全実装済みなのに未クローズと断定できる issue はない。#15/#16/#18 は部分実装、#17/#37/#41/#42 は未検証または未解決、残りは snapshot 上で未実装。

## 範囲と限界

確認は GitHub API の current metadata と `master` snapshot の静的照合。Windows/macOS 実機、workflow run、シグナル・改行の統合試験は実行していない。Issue の内容から原因を確定できないものは「未解決/再現条件不足」とし、アプリ不具合と断定していない。確認時点の `master` SHA は `c87743f52d33d112f0444f8e2a65702a2256d7bc`。
