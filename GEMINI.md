# 🐾 TANUKI プロジェクト共通指示書 (GEMINI.md)

このファイルは、本ワークスペースにおける共通の作法とワークフローを定義するものです。

## 🛠 共通ワークフロー

### 1. 開発日誌の記録 (TanukiJournal)
作業の節目やセッション終了時には、必ず以下の手順で「開発日誌」を更新・反映してください。
- **目的**: 進捗の可視化と、次回セッションへのスムーズな引き継ぎ（Resume）のため。
- **公式日誌の格納先**: `D:\Projects\PyProjects\Documents\Archive\Devlog`
  - 日誌ファイルは `YYYY/MM/開発日誌_YYYYMMDD_タイトル.md` として保存されます。
  - インデックスファイルは `Devlog_Index.md` です。
- **使用ツール**: `D:\Projects\PyProjects\_agent\skills\TanukiJournal\scripts\journal_manager.py`
- **文字コードルール**: 全て **UTF-8** で保存・更新します。
- **実行手順**:
    1. セッション中の変更内容を振り返る。
    2. 以下のコマンドを実行して日誌エントリを生成します。
       ```powershell
       python D:\Projects\PyProjects\_agent\skills\TanukiJournal\scripts\journal_manager.py --title "機能名や作業内容" --summary "概要" --changes "- 変更点1\n- 変更点2" --results "- 結果1" --next "- 次にやること"
       ```
    3. `Devlog_Index.md` に自動登録され、さらに `tanuki-compiler` が自動起動して知識ベースが高速更新されることを確認します。
    4. ※プロジェクト直下の `開発日誌.md` はローカル検証用の一時メモです。こちらもUTF-8で更新してください。

### 2. 知識ベースの再構築 (Rebuild)
日誌の記録以外で、手動で知識ベースを更新したい場合や、ドキュメントの差分更新を行いたい場合は、以下の再構築スクリプトを実行してください。
- **実行コマンド**:
  ```powershell
  python D:\Projects\PyProjects\Documents\Archive\Devlog\rebuild_tanuki.py
  ```
- **処理内容**: `d:\Projects\PyProjects\TANUKI` の `tanuki-compiler` を実行し、変更のあったファイルを高速に差分コンパイルして `knowledge.db` に反映します。

### 3. セッションの再開 (Resume)
新しいセッションを開始した際は、まず最新の知識ベースを探索して過去の経緯や現在の進捗を自動的に「思い出し」、ご主人様に報告してください。
- **実行コマンド**:
  ```powershell
  python D:\Projects\PyProjects\Documents\Archive\Devlog\tanuki_resume.py
  ```
  （または `python D:\Projects\PyProjects\_agent\skills\TanukiJournal\scripts\journal_manager.py --resume`）
- **処理内容**: `TanukiExplorer` が `TANUKI/output_knowledge` の AST 知識木を探索し、最新の進捗や未完了タスク、次にやるべきことを要約して出力します。

## 📁 参照
- スキル詳細: `D:\Projects\PyProjects\_agent\skills\TanukiJournal\SKILL.md`
- 開発日誌保管場所: `D:\Projects\PyProjects\Documents\Archive\Devlog/`
- ナビゲーション: `D:\Projects\PyProjects\_agent\skills\WorkspaceNavigator/`

## 📁 ドキュメントヴォルト管理 (Documents Vault Management)
ドキュメントヴォルト（[Documents](file:///D:/Projects/PyProjects/Documents)）は、一時的な入口（`InBox`）を経由してルールに基づき自動仕分けされるシステムになっています。
詳細およびルールについては [DOCUMENTS_MANAGER.md](file:///D:/Projects/PyProjects/Documents/DOCUMENTS_MANAGER.md) を参照してください。

- **仕分けの実行とTANUKI再構築**:
  `D:\Projects\PyProjects\_maintenance` に移動し、以下のコマンドで仕分けプランを確認・適用できます。
  ```powershell
  cd D:\Projects\PyProjects\_maintenance
  # ステータスと仕分け対象の確認
  python -m documents_manager status
  python -m documents_manager plan --text
  # 仕分けの実行（適用）とTANUKIの自動再構築
  python -m documents_manager apply --rebuild-tanuki
  ```
- **ルールの追加・編集**:
  - 仕分けのルーティングルール: [documents_routing_rules.yaml](file:///D:/Projects/PyProjects/Documents/documents_routing_rules.yaml)
  - RAG（TANUKI）コンパイルポリシー: [documents_rag_policy.yaml](file:///D:/Projects/PyProjects/Documents/documents_rag_policy.yaml)
- **注意点**:
  - 新規作成したドキュメントは、直接格納先に置くか、または `Documents/InBox` に投入して仕分けマネージャを実行してください。
  - `InBox` 配下のファイルは RAG のインデックス対象外となります。

