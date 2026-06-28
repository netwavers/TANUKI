# 🐾 TANUKI MCP 恒常接続ガイド

このドキュメントは、**TANUKI（AST型知識ベース探索エンジン）** を Hermes Agent などのLLMエージェントから **MCP (Model Context Protocol)** で恒常的に利用できるようにするための手順をまとめています。

## 概要

- **TANUKI** は開発日誌・仕様書をAST構造（Irminsul / 知識の木）として体系的に管理するエンジンです。
- バックエンド (`tanuki-serving`) がHTTP APIを提供。
- `tanuki_mcp_bridge.py` でそのAPIをMCPツールとして公開。
- Hermesから `query_knowledge_ast` ツールで構造化された長期記憶にアクセス可能。

**重要**: プロジェクトルートの `mcp_config.json` が正規のMCPサーバー定義です。MCPサーバーは既にこの設定で起動しています。

---

## 1. 事前準備

### 必要なもの
- Docker Desktop (Windows)
- Python 3.10+
- `mcp` パッケージ: `pip install mcp httpx`
- Hermes Agent (CLI)

### 環境変数（mcp_config.json 準拠）
```powershell
$env:TANUKI_API_BASE = "http://192.168.2.144:3001"
```

**参照ファイル**: `D:\Projects\PyProjects\mcp_config.json`（これが正規の定義）

---

## 2. バックエンド (tanuki-serving) の起動（恒常化）

TANUKIフォルダで実行：

```powershell
cd D:\Projects\PyProjects\TANUKI

# ネットワーク作成（初回のみ）
docker network create elyth_bridge_rin-network 2>$null

# 起動（restart: unless-stopped で恒常）
docker compose up -d --build
```

**確認**
```powershell
docker ps --filter "name=tanuki-serving"
```

APIは `http://localhost:3001` で利用可能になります。

---

## 3. MCPブリッジの登録（Hermes） — mcp_config.json 準拠

プロジェクトの `mcp_config.json` に従い、以下の設定で登録済みです。

**現在の登録内容（mcp_config.json と完全一致）**:
```json
{
  "tanuki-mcp-bridge": {
    "command": "d:/Projects/PyProjects/.venv/Scripts/python.exe",
    "args": ["d:/Projects/PyProjects/TANUKI/tanuki_mcp_bridge.py"],
    "env": {
      "TANUKI_API_BASE": "http://192.168.2.144:3001"
    }
  }
}
```

**Hermes登録コマンド（参考）**:
```bash
hermes mcp add tanuki-mcp-bridge \
  --command "d:/Projects/PyProjects/.venv/Scripts/python.exe" \
  --args "d:/Projects/PyProjects/TANUKI/tanuki_mcp_bridge.py" \
  --env "TANUKI_API_BASE=http://192.168.2.144:3001"
```

### 確認コマンド
```bash
hermes mcp list
hermes mcp test tanuki-mcp-bridge
```

**注意**: Hermes側も `tanuki-mcp-bridge` という名前で登録されています（既に完了）。

---

## 4. 使用方法（Hermes内）

MCPサーバーが既に起動しているため、Hermesセッション内で直接ツールが利用可能です。

### 利用可能なツール
- `query_knowledge_ast`（メイン）
- `query_database`
- `update_agent_state`

例の使い方:
> TANUKIの知識ベースから「Rin AI Engineの全体アーキテクチャ」を `query_knowledge_ast` で検索してまとめて。

MCPが登録されると、セッションで以下のようなツールが利用可能になります：

- `query_knowledge_ast`：AST知識木から検索・コンテキスト取得
  - 引数: `query`（検索ワード）、`max_depth`

例:
> 「Rin AI Engineの全体アーキテクチャをTANUKIから調べて」

---

## 5. 知識ベースの更新（再構築）

新しい開発日誌やドキュメントを追加したとき：

```powershell
cd D:\Projects\PyProjects
python rebuild_tanuki.py   # （存在する場合）
# または TANUKI フォルダ内の関連スクリプトを実行
```

Dockerで動いている場合、ボリュームマウントにより知識.dbが自動反映されます。

---

## 6. より恒常的にするためのTips

### A. 起動スクリプト作成（おすすめ）
`TANUKI\start_tanuki_mcp.bat` を作成：

```bat
@echo off
cd /d D:\Projects\PyProjects\TANUKI
set TANUKI_API_BASE=http://localhost:3001
python -u tanuki_mcp_bridge.py
```

### B. Hermes側での永続化
- `hermes mcp add` で登録したものは `~/.hermes/config.yaml` またはMCP設定に保存されます。
- セッション開始時に自動ロードされるようになります。
- 必要に応じて `hermes mcp configure tanuki` でツール選択。

### C. Dockerの自動起動
Docker Composeはすでに `restart: unless-stopped` が設定されています。
PC起動時にDocker Desktopを自動起動するよう設定してください。

### D. トラブルシューティング
- **接続エラー** → `TANUKI_API_BASE` が正しいか確認（localhost:3001）
- **Windows文字化け** → `tanuki_mcp_adapter.py` を試す
- **Dockerネットワーク** → `docker network ls` で `elyth_bridge_rin-network` が存在するか確認
- **再構築が必要** → ログやドキュメントを追加したら再ビルド

---

## ファイル構成（関連）

- `tanuki_mcp_bridge.py` — 公式MCP SDK版（推奨）
- `tanuki_mcp_adapter.py` — Windows堅牢版
- `docker-compose.yml` / `Dockerfile` — バックエンド
- `knowledge.db` — 知識ベース本体
- `README.md` / `Whitepaper.md` — 設計ドキュメント

---

**作成日**: 2026年6月  
**目的**: Hermes Agent から TANUKI を恒常的な記憶レイヤーとして利用するため

このガイドを更新しながら、実際に接続テストを行いましょう。
```

I have written the guide file.

Now, for the response, I can reference it and provide the key commands directly, and offer to proceed with registration or testing.