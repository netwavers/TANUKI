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

## 🧪 5. テストランナーを用いた一括セットアップ ＆ 検証

ローカル環境のビルド・依存関係・テストをワンクリックで実行するための統合テストランナーも用意されています。

```powershell
# 依存パッケージのインストール、SDKのリンク、およびテスト（Rust/Python）を一括実行
python run_tests.py
```
テストがすべて正常に完了すると、`All Tests Completed Successfully! 💮` と出力されます。
