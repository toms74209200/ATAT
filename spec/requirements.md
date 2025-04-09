# ATAT 技術仕様書

「ATAT」は、TODO.mdファイルとGitHub Issuesを同期するRustで実装されたCLIアプリケーションです。

## 技術スタック

- **言語**: Rust 1.86.0
- **依存クレート**:
  - `clap`: コマンドライン引数の解析（サブコマンド構造の実装）
  - `reqwest`: HTTP通信
  - `tokio`: 非同期ランタイム
  - `serde` & `serde_json`: JSON操作
  - `pulldown-cmark`: Markdownのパース
  - `keyring`: OSのセキュアストレージを利用した認証情報の管理
  - `toml`: 設定ファイル操作（必要に応じて）
  - `anyhow`: エラーハンドリング
  - `log` & `env_logger`: ロギング

