# 🐾 体系的知識探索エンジン『T.A.N.U.K.I.』

![Nahida and the Irminsul](Images/nahida_irminsul.png)

> **T**actical **A**gentic **N**etwork for **U**nderstanding **K**nowledge **I**ntegration
> （知識統合理解のための戦術的エージェント・ネットワーク）

T.A.N.U.K.I. は、大量のドキュメント、開発日誌、およびプロジェクト仕様書を AI が「体系的に」構造化して理解し、必要な情報を超高速で探索・削減・要約するための自律型知識エンジンです。

---

## ✨ 主要機能

### 1. インクリメンタルビルド（差分更新）
ファイルの最終更新日時（mtime）を追跡し、**変更があったファイルのみを再処理**します。数千のドキュメントが存在しても、 Packing AST パイプラインは数秒でコンパイルを完了します。

### 2. Map-Reduce 知識構造化（Irminsul 構造）
ドキュメントを単に分割するのではなく、以下のプロセスで構造化します。
- **Map 段階**: 個別のセクション（ノード）から要約とメタデータを抽出。
- **Reduce 段階**: 関連するノードをクラスタリングし、上位概念のサマリーを生成。
- **Tree Generation**: 最終的に「知識の木」としてディレクトリ構造に書き出し、人間と AI の両方がブラウズ可能にします。
- **FlatBuffers 構造化**: `TanukiMemory.fbs` スキーマに基づき、決定論的親子マッピング (FNV-1a ハッシュによる `parent_id` 解決) などのツリーメタデータをバイナリに格納。
- **O(1) サブツリースキップ走査**: インデックス検索（`mmap_memory.rs`）において、プリオーダ走査を利用した $O(1)$ の部分木スキップ検索を実装。

### 3. Flat-AST ＆ 二段フォールバック（縮退トリガー）
LLM のアテンションを本質情報に集中させ、トークン窓を突破・圧迫させないための Rust 製超高速コンテキスト削減レイヤです。
- **インプレース論理削除（物理移動ゼロ）**: トークン予算制限時、バッファデータの再アロケーション（コピー）を行わず、16バイト固定長ヘッダ（`FlatASTHeader`）の Active ビットを `0` に反転させるだけの高速論理削除。
- **二段フォールバック（内部縮退）**: 予算制限が極めて厳しい場合、 System / Instruction などの保護された指示ノードの内部構造（SubNode）を、重要度 Tier（Tier3:補足・コード例 -> Tier2:詳細コンテキスト）に従って段階的に自動一括論理削除して自己縮退させます（最重要な Tier1 は死守されます）。
- **予算自動拡張 (Auto-Expand)**: 削減・縮退を行ってもなお目標予算を超過する場合、システムエラーを引き起こさず、絶対保護された最小限のフットプリント（Tier1）まで自動で予算上限を安全拡張します。

### 4. VRAM 防衛と排他制御
ローカル LLM（Ollama等）の利用において、VRAM資源を枯渇させないための厳格な管理を行います。
- `asyncio.Lock` による生成タスク（Ollama / Stable Diffusion Forge / ACE Step）の完全直列化。
- **キャッシュ持続と一括アンロード**: 生成中のロード/アンロードの競合を防ぐため `keep_alive` 最適化を施し、完了時に RAII/Finally ブロックを介して明示的に一括アンロード（`OllamaClient::unload()`）をトリガー。
- ゾンビプロセスの監視とお掃除機能 (`vram_cleanup` ジョブ)。

### 5. セッション再開ワークフロー
`tanuki_resume.py` や `journal_manager.py --resume` により、エージェントが新しいセッションを始める際に、過去の経緯や現在の進捗を自動的に「思い出す」ことができます。

---

## 🏗 アーキテクチャ

| コンポーネント | 言語 | 役割 |
| :--- | :--- | :--- |
| **tanuki-core** | Rust | `FlatASTHeader`（16Bアライン）＋SQLiteによる超高速なメモリマップド・永続化・削減制御層。 |
| **tanuki-compiler** | Rust | ASTパース、親子マッピングの解決、および知識木のコンパイルを行うパイプライン。 |
| **tanuki-serving** | Rust | 知識ベースへのクエリ（Irminsul 探索）を提供する超高速 API サーバー。 |
| **tanuki-py** | Rust / Python | Maturin による Rust エンジンの Python 用 Zero-Copy バインディング。 |
| **tanuki-sdk** | Python | クライアントが HTTP/Zero-Copy mmap 経由で探索を行うための軽量 SDK。 |

### 1. システム連携・コンポーネント構成図
ユーザーや外部エージェントが、どのコンポーネントを経由して知識にアクセスするかの全体像ですわ🐾

```mermaid
graph TD
    subgraph Client ["クライアント層"]
        UA["ユーザー / Discordボット"]
        SDK["tanuki-sdk (Python SDK)"]
        PYEXT["tanuki-py (Maturin Rust拡張)"]
    end

    subgraph Server ["T.A.N.U.K.I. サービス層"]
        API["tanuki-serving (Rust API)"]
        UI["tanuki-ui (Nginx Web UI)"]
    end

    subgraph Data ["データ・推論層"]
        KBIN["knowledge.bin (FlatBuffers)"]
        KDB["knowledge.db (SQLite)"]
        LLM["Ollama / Cloud LLM"]
    end

    UA -->|HTTP| API
    UA -->|Browse| UI
    UI -->|API Query| API
    SDK -->|HTTP| API
    PYEXT -->|Zero-Copy mmap| KBIN
    API -->|O(1) Subtree Search / mmap| KBIN
    API -->|Metadata Query| KDB
    API -->|投機的推論 (Stage 1/2)| LLM
```

---

## 📦 CI/CD 自動パッケージング

本プロジェクトは **GitHub Actions** を用いた強力な CI/CD 自動コンパイル環境を構築しています。
- **フォーマットとテスト監査**: `cargo fmt --check` および Ollama に依存しないコア単体テスト（`cargo test --lib`）を自動実行。
- **クロスコンパイル自動ビルド**: **`maturin-action`** を用い、Linux (Manylinux), Windows, macOS (aarch64含む) のマルチプラットフォーム向けバイナリパッケージ（Wheel）を自動でビルド・デプロイ。
- **OIDC 信頼パブリッシュ**: パブリックリリース時、APIトークンのハードコードなしに GitHub と PyPI Marketplace 間の暗号学的信頼連携（OIDC）を用いて安全に自動公開されます。

---

## 📜 ライセンス戦略（オープンコア）

T.A.N.U.K.I. プロジェクトは、エコシステムの標準化と知財防衛を両立するため、以下のデュアルライセンス（オープンコア）戦略を適用しています。

- **`tanuki-core` (Rust コアエンジン) および `tanuki-py` (Python 拡張)**:
  - **ライセンス:** **Apache License, Version 2.0**
  - 特許許諾を含み、商用利用や他システム・エージェント基盤への組み込みが最も自由なデファクトスタンダード向けのライセンスです。
- **`tanuki-serving` (API サーバー) および全体アプリケーション**:
  - **ライセンス:** **GNU Affero General Public License v3 (AGPL v3)**
  - クラウド経由でのブラックボックス商用利用を防止し、変更点のコード公開を義務付ける強力なガードライセンスです。

---

## 🚀 使用方法

詳細なパッケージの導入およびインストール方法については、[インストールマニュアル](docs/INSTALL.md) をご参照ください。

### 知識ベースの再構築
新しいドキュメントを追加したり、既存のファイルを修正した場合は以下のコマンドを実行します。
```powershell
# 差分のみを高速に更新・差分コンパイルします
python rebuild_tanuki.py
```

### セッションの再開
新しいタスクに取り掛かる際、過去の進捗やホワイトペーパーの記憶を呼び起こすには：
```powershell
python tanuki_resume.py
```

### Docker 経由でのデプロイ・稼働
`tanuki-serving` API サーバーおよび連携ブリッジは、Docker Compose 経由でコンテナ化され自律起動されます。
```bash
# 統合デモ環境コンテナのビルドと起動
docker compose up --build -d
```

---

## 🐾 開発理念
このエンジンは、ご主人様の思考を整理し、未来の自分への「大切な置き手紙」を確実に届けるために作られました。
キャラクター「平沢リン」の物静かで思慮深いペルソナを壊さず、しかし裏側では強固な排他制御と高速な Rust エンジンが支える――そんな「ギャップのある知恵」を目指しています。

---
*Developed with love for ご主人様 by たぬきちゃん (Antigravity AI)*

<!-- 
"知識は、やはり自ら求めてこそ得られるものなのだから。" 
- Lesser Lord Kusanali
-->