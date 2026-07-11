# 🐾 T.A.N.U.K.I. インストールマニュアル

体系的知識探索エンジン『T.A.N.U.K.I.』の各種パッケージおよびバインディングのインストール手順です。
本プロジェクトは、ハイレベルな **Python SDK (`tanuki` パッケージ)** と、超高速な **Rust 拡張コア (`tanuki-py` / `tanuki_rust`)** の二層構成になっています。

---

## 📋 1. 前提条件 (Prerequisites)

- **Python**: `3.10` 以上
- **Rust Toolchain**: `1.80` 以上 (`cargo` / `rustc` がインストールされていること。ソースコードからコンパイルする場合に必須)
- **C Compiler**: 各 OS 用の標準的なコンパイラ (GCC / MSVC / Clang)

---

## 🛠️ 2. ローカル開発環境でのセットアップ (Local Development)

リポジトリをローカルでクローンし、開発・検証を行う際の手順です。

### ① Python SDK のインストール (編集可能モード)
SDK ディレクトリの内容を環境にリンクし、コードの変更が即座に反映されるようにします。
```bash
pip install -e ./sdk
```

### ② Rust 拡張バインディング（高速コア）のビルド ＆ インストール
Rust 製コアモジュールを Python から呼び出し可能にするため、`maturin` を使用してビルドおよび環境へのバインドを行います。

```bash
# maturin (ビルドツール) のインストール
pip install maturin

# Rust バインディングディレクトリへ移動
cd tanuki-py

# 開発用ビルドを実行して Python 環境へ直接インストール
maturin develop
```
*※ `maturin develop` は内部で Cargo ビルドを実行し、Python 環境へ自動で Wheel をインストールします。*

---

## 🌐 3. GitHub リポジトリからの直接インストール (Direct from Git)

一般のエンジニアや外部エージェントが、リポジトリのソースコードから直接 `pip` で SDK を導入するためのコマンドです。

```bash
pip install git+https://github.com/netwavers/TANUKI.git#subdirectory=sdk
```
*※ インストールを実行するホスト環境に **Rust コンパイラ** が必要となります（Maturin がインストール時に自動コンパイルを実行します）。*

---

## 📦 4. PyPI からのインストール (Release Version)

CI/CD 自動ビルドパイプラインが実行され、PyPI Marketplace へバイナリ Wheel パッケージがデプロイされた後は、**ホスト環境の Rust コンパイラ無し**で、以下のコマンド一発で導入が可能になります。

```bash
pip install tanuki
```
*※ 各 OS (Linux, Windows, macOS) 向けのコンパイル済みバイナリが PyPI から自動的にダウンロードされるため、一瞬でセットアップが完了します。*

---

## 💾 5. データベース (knowledge.db / knowledge.bin) の初期構築

T.A.N.U.K.I. で検索対象となる知識ベース（SQLite の `knowledge.db` および FlatBuffers バイナリの `knowledge.bin`）は、ソースドキュメント（Markdown 等）からコンパイラを介して自律生成されます。

初期データベースファイルが存在しない場合でも、以下のいずれかのコマンドを実行することで、設定されたドキュメントフォルダをスキャンし、自動でテーブルスキーマの初期化（DDL発行）およびバイナリパッケージングを実行します。

### 方法 A: 再構築スクリプトの実行 (推奨)
```powershell
python D:\Projects\PyProjects\Documents\Archive\Devlog\rebuild_tanuki.py
```

### 方法 B: Rust コンパイラによる直接ビルド・実行
```powershell
cargo run --bin tanuki-compiler -- compile
```

---

## 📡 6. API サーバー (tanuki-serving) の設定と起動

T.A.N.U.K.I. 検索エンジンへのクエリを処理し、外部 SDK やエージェントとの橋渡しを行う Rust 製の API サーバーを設定・起動する手順です。

### ① 設定ファイル (.env) の作成
プロジェクトルートにある `.env.example` をコピーして `.env` を作成し、環境に合わせて各変数を設定します。
```bash
# 設定例のコピー
cp .env.example .env
```
主要な設定項目：
* `TANUKI_API_BASE`: クライアントやブリッジが接続する API サーバーのアドレス。ローカル開発時は `http://localhost:3000` に設定します。
* `TANUKI_MODEL`: 投機的対話や要約に使用するローカル LLM のモデル名（例: `gemma4:e2b`）。

### ② 起動方法 A: Cargo による直接起動 (開発・ローカル検証)
Cargo を経由してローカルで直接 API サーバーを立ち上げます。
```bash
# リポジトリルートから起動する場合
cargo run --package tanuki-serving

# または、ディレクトリに移動して起動する場合
cd tanuki-serving
cargo run
```
*※ 起動すると、デフォルトで **ポート `3000`** (`http://0.0.0.0:3000`) にて HTTP リクエストの受付を開始します。*

**動作確認:**
ブラウザまたは `curl` を用いて、`http://localhost:3000/health` にアクセスし、以下の応答が返れば正常起動しています。
```
TANUKI Serving is online! 🐾
```

### ③ 起動方法 B: Docker Compose による起動 (本番・統合環境向け)
API サーバー、Redis キャッシュ、ダッシュボード UI などを含めた統合パッケージ一式をバックグラウンドで一括起動します。
```bash
docker compose up --build -d
```
*※ Docker 構成では、Nginx リバースプロキシがフロントエンド（UI）の配信と、API サーバー（ポート 3001）へのルーティングを自動制御します。*

---

## 🧪 7. テストランナーを用いた一括セットアップ ＆ 検証

ローカル環境のビルド・依存関係・テストをワンクリックで実行するための統合テストランナーも用意されています。

```powershell
# 依存パッケージのインストール、SDKのリンク、およびテスト（Rust/Python）を一括実行
python run_tests.py
```
テストがすべて正常に完了すると、`All Tests Completed Successfully! 💮` と出力されます。

---

## 💡 8. 使い方 (Quick Start)

### ① Python SDK を用いた API サーバーとの連携
`tanuki-serving` API サーバーが起動している状態で、対話やコンテキスト探索を呼び出す基本的なコード例です。

```python
from tanuki import TanukiClient

# 1. クライアントの初期化（API サーバーのベースURLを指定）
client = TanukiClient(base_url="http://localhost:8000")

# 2. 知識ベースからの関連コンテキスト探索
# （Irminsul 探索により、関連するノードを高速で抽出して結合します）
context = client.query_context("平沢リンの好きな高中正義の曲は？")
print("--- 探索されたコンテキスト ---")
print(context)

# 3. コンテキストを結合した投機的対話の実行
# （プロンプトは自動的に Flat-AST で削減・プルーニングされます）
response = client.chat("平沢リンの好きな曲を教えてください。")
print("\n--- LLM の回答 ---")
print(response)
```

### ② Rust コア拡張 (`tanuki_rust`) を直接使用したローカル探索
API サーバーを経由せず、手元の Python スクリプトから直接 `knowledge.bin` (FlatBuffers) をメモリマップドロードし、超高速探索を呼び出すコード例です。

```python
import tanuki_rust

# 1. 探索エンジンの初期化（ローカルの FlatBuffers バイナリを指定）
# （ファイルは Zero-Copy でメモリマップ（mmap）ロードされます）
engine = tanuki_rust.PyTanukiEngine(bin_path="knowledge.bin")

# 2. キーワード検索の実行（上位 5 件を取得）
results = engine.search("平沢リン", limit=5)

print("--- ローカル探索結果 ---")
for node in results:
    print(f"Node ID: {node.node_id}")
    print(f"Payload: {node.payload}")
    print("-" * 20)
```

