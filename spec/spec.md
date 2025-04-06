TODO.md の内容を GitHub の Issues に反映するCLIアプリケーション

git と同様の操作体系

TODO.md の内容を GitHub の Issues に push する

```bash
atat push
```

- TODO.md にある未チェックの項目が GitHub の Issues に登録されていないとき, Issue を新規作成する
- GitHub の Issues にある open な Issue のうち, TODO.md にある項目がチェックされているものは, Issue をクローズする

GitHub の Issues から TODO.md の内容を更新する

```bash
atat pull
```

- GitHub の Issues にある open な Issue が TODO.md にないとき, TODO.md に追加する
- TODO.md にある未チェックの項目が GitHub の Issues ではクローズされているとき, TODO.md の項目をチェックする

未検討の項目:

1. 項目の識別と対応付け
- TODO項目とGitHub Issueの対応関係をどのように管理するか
- TODO.md内でのIssue番号の表現方法
- 既存のIssueとの紐付け方法

2. コンフリクト解決
- TODO.mdとGitHub Issuesで異なる状態になった場合の解決方法
- 同期の優先順位（GitHub優先かTODO.md優先か）
- コンフリクト発生時のユーザーへの通知方法

3. Issue内容の同期範囲
- Issueのタイトル以外の情報（説明、ラベル、担当者等）の扱い
- TODO.md内での付加情報の表現方法
- 同期対象とする情報の範囲

4. TODO.mdの構造
- カテゴリ分けやネスト構造の扱い
- 構造化情報のGitHub Issues側での表現方法
- 階層構造の同期方法

5. 認証と権限
- GitHub APIの認証方法
- 必要な権限スコープ
- 認証情報の保存方法

6. リポジトリ設定
- 同期対象のリポジトリ指定方法
- 複数プロジェクトでの設定管理
- リポジトリ設定の保存場所

7. パフォーマンスとスケーラビリティ
- 大量のIssue存在時の処理方法
- APIレート制限への対応
- 同期処理の効率化（増分同期等）

8. エラー処理
- ネットワークエラー時の挙動
- API制限到達時の対応
- 同期失敗時のリカバリ方法
- 不整合発生時の検出と修復