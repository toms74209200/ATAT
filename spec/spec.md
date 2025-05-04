TODO.md の内容を GitHub の Issues に反映するCLIアプリケーション

git と同様の操作体系

## TODO.md の内容を GitHub の Issues に push する

```bash
atat push
```

- TODO.md にある未チェックの項目が GitHub の Issues に登録されていないとき, Issue を新規作成する
- GitHub の Issues にある open な Issue のうち, TODO.md にある項目がチェックされているものは, Issue をクローズする

1. TODO.mdの項目がGitHub Issuesにない場合

 - 新規Issueを作成
 - 作成したIssue番号をTODO.mdに追記

2. TODO.mdの項目がGitHub Issuesに既にある場合（タイトルが一致）

 - 既存のIssue番号をTODO.mdに追記

##GitHub の Issues から TODO.md の内容を更新する

```bash
atat pull
```

- GitHub の Issues にある open な Issue が TODO.md にないとき, TODO.md に追加する
- TODO.md にある未チェックの項目が GitHub の Issues ではクローズされているとき, TODO.md の項目をチェックする

## Issue内容の同期範囲

以下の情報のみを同期対象とする:
- タイトル: TODO.mdの項目テキストとIssueのタイトルを同期
- 状態: TODO.mdのチェック状態とIssueのopen/closed状態を同期
- Issue番号: TODO.mdの項目に対応するIssue番号を記録

## TODO.mdの構造

- 階層構造（ネスト）は扱わない。すべての項目をフラットな構造として扱う
- チェックボックス形式の項目のみを同期対象とする

## 実装

GitHub アプリとして実装し実行時にユーザーから権限を取得する

https://docs.github.com/ja/apps/creating-github-apps/writing-code-for-a-github-app/building-a-cli-with-a-github-app

- GitHub APIの認証方法
  - デバイスフローを使用したユーザー認証
    1. 初回実行時（atat login）:
       - CLIがデバイスフローを開始
       - ユーザーに認証URLとデバイスコードを表示
       - ユーザーがブラウザでGitHubにアクセスしコードを入力
       - ユーザーがアプリケーションの権限を承認
       - CLIがユーザーアクセストークンを取得・保存
    2. 2回目以降:
       - 保存されたユーザーアクセストークンを使用
       - トークンが無効な場合は再ログインを要求
- 必要な権限スコープ
  - `issues` (read/write): Issuesの作成、更新、クローズ
  - `contents` (read): TODO.mdファイルの読み取り
- 認証情報の保存
  - ユーザーアクセストークン: ~/.config/atat/token に保存
  - 設定ファイルのパーミッション: 600 (所有者のみread/write可能)
- コマンド
  - `atat login`: デバイスフローによる認証を開始
  - `atat logout`: 保存されたトークンを削除
- コマンド出力例
  ```
  $ atat login
  ! First copy your one-time code: XXXX-YYYY
  - Press Enter to open github.com in your browser... 
  ✓ Authentication complete. ATAT has been granted access to:
    - Read repository contents
    - Read and write issues
  ✓ Logged in as <username>
  ```

  ```
  $ atat push
  X Authentication required
  ℹ To get started, please run:  atat login
  ```

  ```
  $ atat logout
  ✓ Logged out of github.com
  ```

## リポジトリ設定
- リポジトリ設定は git remote のように以下のサブコマンドで管理する:
  ```bash
  # リポジトリの追加
  $ atat remote add owner/repo
  ✓ Repository owner/repo has been added

  # 現在の設定を表示
  $ atat remote
  owner/repo

  # リポジトリの削除
  $ atat remote remove owner/repo
  ✓ Repository owner/repo has been removed
  ```
- 設定は ~/.config/atat/config.json に保存
- 複数プロジェクトの場合は、.git/config のように、.atat/config でプロジェクト固有の設定を上書き可能
