# Issue #37 / #42 調査

調査日: 2026-10-08 (UTC)  
対象コード: Twil3akine/with の作業ブランチ investigate/37-42-auth、基準コミット c87743f。GitHub Issue の内容・コメントは同日に GitHub connector で取得した。Issue レコードの記録上の更新日は #37 が 2025-12-30、#42 が 2026-01-21。#37 のコメント取得結果に作成日時は含まれなかった。

## 結論

この情報だけでは #37 と #42 が同じ原因だとは確定できない。両方とも git push と認証に関連するが、#37 は「毎回 username/password (token) を求められる」、#42 は「git push を処理できず、git add . などはできる」という別の症状記述である。#37 へのメンテナーコメントは credential helper 未設定の可能性を挙げているが、本人も未再現と明記しており、診断結果ではない。#42 には追加コメントがない。両 issue とも OS、With/Git の版、remote の transport、実際のエラー出力、Git 設定がない。

現在のコードからは、With が認証情報や標準入出力を独自に遮断する挙動は見つからない。with git の REPL 内で push を実行すると、パーサーは git と push を実行対象にし、executor は git を子プロセスとして直接起動する。executor に標準入出力の差し替えはなく、WITH_CONTEXT_STACK 以外の環境変数も明示的には変更・削除していない。ただし、これだけでは報告者の実環境で Git が同じ資格情報ヘルパー、askpass、SSH agent、Git 実行ファイルを使ったことまでは証明できない。

## Issue の記載

- [#37](https://github.com/Twil3akine/with/issues/37) は push のたびに username と token を求められると報告し、terminal screenshot を添付している。画像は認証情報を含む可能性を避けるため本調査では開いていない。GitHub connector 取得時点でコメントは1件。コメントは credential helper 未設定を可能性として示し、git config --global credential.helper と環境情報の確認を求めている。これはメンテナーの仮説であり、実行結果ではない。
- [#42](https://github.com/Twil3akine/with/issues/42) は git push は処理できない一方、git add . 等はできると記載し、投稿者自身が「認証情報アクセス」の問題かもしれないと推測している。GitHub connector 取得時点でコメントは0件。

git add の成功は remote 認証を検証しない。ローカル index を更新する操作であり、push の remote transport/認証が失敗しても成功し得る。したがって #42 の記載は Git の push 固有経路の問題と矛盾しないが、With の不具合か、Git の設定・transport・remote 側の問題かを切り分ける証拠にはならない。

## コード経路と確認できたこと

ソース参照は基準コミット c87743f の行番号。

1. with git の起動引数は src/main.rs 181–205 行で TargetContext にし、REPL を開始する。REPL は入力を解析して外部実行を execute_child_process に渡す (src/main.rs 108–127 行)。
2. src/parser.rs 105–119 行は通常コマンドを Execute { program, args } にする。ターゲットコンテキストが git なら、入力 push は実質 git push になる。! 付き実行も外部コマンド経路だが、通常の push には不要。
3. src/executor.rs 50–67 行は process::Command::new(program_path) と .args(args) でプロセスを作成し、.spawn() する。ここに shell 起動、stdin/stdout/stderr の差し替え、Git 専用の設定変更はない。Rust Command の既定では標準 IO は親から継承される。
4. src/executor.rs 56–65 行は WITH_CONTEXT_STACK の値を計算して子に設定するだけで、env_clear や他の env_remove 呼び出しはない。従って、With に渡された環境は子に継承される。GIT_ASKPASS、SSH_ASKPASS、SSH_AUTH_SOCK、GIT_TERMINAL_PROMPT はコードで特別扱いされず、親に存在すれば子環境に残る。実際に存在したか、Git/SSH がそれを使ったかは不明。
5. Unix では resolve_program は "git" をそのまま返す (src/executor.rs 12–15 行)。Windows では which でパス解決を試みる (4–10 行)。いずれも shell alias/function を呼び出すコードではない。利用者の shell で git が alias/function などを通じて別の設定を付加している場合は、With 内の git と異なる可能性があるが、該当設定の有無は不明。
6. 子の終了コードは、127 の場合に限って親も終了する処理がある。通常の非ゼロ終了コードは親が表示し直さない (src/executor.rs 67–86 行)。Git 自身の stderr は継承先へ出るコード経路だが、Issue の「cannot process」がどの表示・終了状態を指すかは不明。

直接 git push は起動元 shell から Git を実行する一方、With 内の push は With の REPL から子 Git を直接起動する。With は shell を間に挟まない。起動元で PATH、Git の選択、TTY 接続、環境設定が異なれば結果に影響し得るが、報告者の実環境については観測されていない。コードから言えるのは With 側が親から渡された環境と標準 IO を通常継承する設計であることまで。

| 項目 | 直接 git push | with git 内の push | この調査で分かった範囲 |
|---|---|---|---|
| 起動 | 起動元 shell が Git を選択・起動 | With が git を Command で直接起動 | With 内では shell alias/function を経由しない。利用 shell の設定差は未確認 |
| stdin/stdout/stderr | 起動元 shell の FD | With の子プロセスは既定で親 FD を継承 | コード上のリダイレクトなし。実端末での測定は未実施 |
| TTY | 起動元 shell の FD 次第 | With の標準 FD が TTY なら子もその FD を共有 | PTY 上の偽 Git probe では両経路の stdin/stdout/stderr が TTY。端末モードと報告者の状態は不明 |
| credential helper | Git 自身の設定・transportに従う | 同じ Git の仕組み。With 独自の helper 処理なし | 現在の利用者設定は未確認 |
| askpass / SSH agent | 環境・Git/SSH 設定に従う | With は対象変数を消さない | 存在と実利用は未確認 |
| 環境変数 | shell が起動 Git に渡す環境 | With が継承し、WITH_CONTEXT_STACK を設定 | HOME, PATH, GIT_*, SSH_* の実値は調査・開示していない |

## 実験と再現性

- ローカル Git の隔離実験を実施した。使い捨て HOME と一時 Git 設定に synthetic credential helper を設定して git credential fill を実行したところ、終了コード 0、helper の get 呼び出しありを確認した。helper のダミー username/password は stdout に出さず破棄し、一時ディレクトリは終了時に削除した。この実験が示すのは、この実行環境の Git が明示設定された helper を呼べることだけであり、With 内の実 push、報告者の設定、remote 認証を再現していない。
- PATH の先頭に一時的な偽 git 実行ファイルを置き、PTY 上で直接 git push と、ビルドした With から push を同じ起動環境・作業ディレクトリで呼び出した。両方で偽 Git が argv `push` を受け取り、stdin/stdout/stderr が TTY と認識され、合成した GIT_ASKPASS / SSH_ASKPASS / SSH_AUTH_SOCK / GIT_TERMINAL_PROMPT の「値あり」フラグが残ることを確認した。With 側では加えて WITH_CONTEXT_STACK が設定されていた。値と資格情報はログに書いていない。この probe は実 Git の認証処理や terminal mode を試しておらず、利用者環境での再現ではない。終了入力は製品仕様どおり全終了コード 127 となった。
- 最初の cargo test はツールチェーン準備前に cargo: command not found となったが、toolchain/env の提供後に再試行し、61件すべて成功した。With バイナリの build も成功した。
- 実 remote への push は行っていない。資格情報や利用者の設定値を読む実験もしていない。

したがって、プロセス起動・FD・環境変数継承は偽 Git で比較できたが、両 issue の認証挙動を同一条件で比較する再現は未完了。Git の stderr / 終了コード、With の版、OS/shell、HTTPS/SSH の別、remote URL の種類、Credential helper 設定の有無、askpass/agent の有無、With と直接実行で選ばれる Git バイナリが不明である。

## 最小の次の確認

報告者の環境で、同じ作業ディレクトリ・同じ Git バイナリ・同じ remote を用い、資格情報そのものを記録せず、直接 git push と with git 内の push の両方について次だけを比較する必要がある。

1. OS、shell、With/Git の版、remote が HTTPS/SSH のどちらか。
2. それぞれの Git の終了コードと伏字済み stderr。プロンプトが出るか、認証拒否か、transport/設定エラーかを区別する。
3. git config --show-origin --get-all credential.helper の有無（出力に秘密が混ざらないことを確認して共有）。加えて GIT_ASKPASS / SSH_ASKPASS / SSH_AUTH_SOCK / GIT_TERMINAL_PROMPT は値を出さず、設定の有無だけを確認。
4. shell alias/function/wrapper の有無と、直接実行・With 内それぞれの git 解決先の同一性。PATH の全文や資格情報は共有しない。

これらを取得できれば、認証情報プロンプト、Git の認証拒否、With の起動/引数問題を最小限で分けられる。現時点で特定の修正を提案する根拠はない。

## 参照

- [Issue #37](https://github.com/Twil3akine/with/issues/37) / [コメント #3698863306](https://github.com/Twil3akine/with/issues/37#issuecomment-3698863306)
- [Issue #42](https://github.com/Twil3akine/with/issues/42)
- [src/main.rs @ c87743f](https://github.com/Twil3akine/with/blob/c87743f/src/main.rs#L108-L127)
- [src/main.rs @ c87743f: CLI 起動引数](https://github.com/Twil3akine/with/blob/c87743f/src/main.rs#L181-L205)
- [src/parser.rs @ c87743f](https://github.com/Twil3akine/with/blob/c87743f/src/parser.rs#L105-L119)
- [src/executor.rs @ c87743f](https://github.com/Twil3akine/with/blob/c87743f/src/executor.rs#L4-L15)
- [src/executor.rs @ c87743f: child process](https://github.com/Twil3akine/with/blob/c87743f/src/executor.rs#L50-L87)

