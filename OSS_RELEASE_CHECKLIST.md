# 🐾 T.A.N.U.K.I. OSS化リリース・チェックリスト

TANUKIエンジンのオープンソース（OSS）公開に向けたタスクリストと現在の進捗ステータスですわ！ご主人様と一緒に一つずつクリアしていきましょうね🐾

---

## 📊 全体進捗

- [x] **Phase 1: コア分離（Decoupling）** `[100%]`
- [/] **Phase 2: 再現性担保（Portability）** `[100%]`
- [ ] **Phase 3: エコシステム構築（Public Release）** `[40%]`

---

## 🛠️ Phase 1: コア分離 (Decoupling) `[100%]`
ご主人様のプライベートデータや固有環境を隠蔽し、エンジン単体をポータブルにするフェーズです。

- [x] **独自データ・プライベートプロンプトの完全な外部インジェクション化**
  - **内容:** ディレクトリパスやAPIキーを外部から環境変数で与える設計。
  - **実績:** [.env.example](file:///D:/Projects/PyProjects/TANUKI/.env.example) の用意、および Rust コアの動的環境変数ロード機能の実装。
- [x] **TanukiParser のEBNF定義ファイルをパッケージとして独立抽出**
  - **内容:** カスタム構文解析モジュールを完全に独立化。
  - **実績:** `D:\Projects\PyProjects\TanukiParser` プロジェクトとして別リポジトリに分離完了。
- [x] **2-Stage推論パイプラインのインターフェース抽象化**
  - **内容:** LLMプロバイダ（Ollama、Geminiなど）に依存しない抽象定義.
  - **実績:** `llm_manager` 共通ライブラリを統合し、モデル選択と呼び出し層を動的に解決可能に。

---

## 📦 Phase 2: 再現性担保 (Portability) `[100%]`
世界中の開発者が、手元のローカル環境で「今すぐ動かせる」再現性を担保するフェーズです。

- [x] **FlatBuffersによる高速バイナリシリアライズと mmap 探索**
  - **内容:** メモリマップド検索に適した高速バイナリフォーマット定義と出力。
  - **実績:** `knowledge.bin` の出力完了。Maturin による Rust 拡張 `tanuki-py` および Pure Python SDK (`tanuki-sdk`) のビルド・テストが成功！
- [x] **対話型 TUI 構成ツールの提供**
  - **内容:** 依存パッケージなしで手軽に `.env` の書き換えや設定確認ができるコマンド。
  - **実績:** [tanuki_config.py](file:///D:/Projects/PyProjects/TANUKI/tanuki_config.py) の実装完了。
- [x] **外部ライブラリ依存関係（OpenSSLなど）の排除**
  - **内容:** Linux/Windows環境でビルド時に OpenSSL エラーが起きないようにする。
  - **実績:** Rust 側の TLS ライブラリを `rustls-tls` に切り替え、静的バイナリビルド時の安全性を確保。
- [x] **Docker Composeによる統合デモ環境構築**
  - **内容:** `docker compose up` 一発で、Ollama（LLM自動プル）＋ `tanuki-serving` ＋ シミュレーション用 UI のセットが立ち上がる環境 of 構築。
  - **実績:** カスタムブリッジネットワークとDNSのバインド、および自動プルコンテナ（`ollama-puller`）、Nginx Web UI配信サービスを含めた `docker-compose.yml` の定義とWSL上での動作検証完了。
- [x] **テスト自動化とカバレッジ確保**
  - **内容:** CI/CDおよびローカルで確実に動作する自動テストスイートの拡充。
  - **実績:** `sdk/tests/test_sdk.py` (pytest/pytest-asyncio 結合テスト) および `tanuki-core/src/db.rs` (SQLiteインメモリユニットテスト) を実装。一括で自動テストを実行する unified runner (`run_tests.py`) を構築し、全テスト合格を確認。

---

## 🚀 Phase 3: エコシステム構築 (Public Release) `[40%]`
世界中のコミュニティに向けて公式に公開し、デファクトスタンダードにするための整備フェーズです。

- [x] **基本 README ドキュメントの整備**
  - **実績:** [README.md](file:///D:/Projects/PyProjects/TANUKI/README.md) に主要機能やアーキテクチャの概要を記述済み。
- [x] **詳細データフロー・アーキテクチャ図の掲載**
  - **内容:** ASTの生成、Reduce処理、および FlatBuffers 構造のデータフローを視覚的に解説する。
  - **実績:** `README.md` にシステムコンポーネント構成図およびMap-ReduceデータフローのMermaid図を追記。
- [ ] **ライセンス決定と付与**
  - **内容:** 普及優先の Apache 2.0、またはマネージドサービス商用化を見据えた AGPL v3 / オープンコアなどのライセンス戦略の最終確定。
- [ ] **GitHub Actionsによる CI/CD パイプラインの構築**
  - **内容:** Rustコア、Python SDKの自動テスト、および Maturin による PyPI リリースパッケージ (Wheel) ビルドの自動化。
- [ ] **コントリビューションガイド（CONTRIBUTING.md）およびコード規約の整備**

---
*Created by たぬきちゃん (Antigravity AI) for ご主人様*
