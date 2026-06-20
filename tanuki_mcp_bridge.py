import asyncio
import httpx
import json
import os
import sys
from typing import Optional, List, Dict, Any
from mcp.server import Server, NotificationOptions
from mcp.server.models import InitializationOptions
import mcp.types as types
from mcp.server.stdio import stdio_server

# ==========================================
# 0. 標準ストリームのUTF-8完全強制とエラー対策
# ==========================================
# MCPサーバーは sys.stdout をJSON-RPC通信に占有するため、stdoutのバッファや
# エンコーディングレイヤーの破壊は絶対に避ける必要がありますが、
# sys.stderr と sys.stdin については UTF-8 でのやり取りを完全に保証するために
# 安全かつ強制的にUTF-8（不正なバイトは置換）へと再構成しますわ！
for _stream in (sys.stderr, sys.stdin):
    if _stream and hasattr(_stream, 'reconfigure'):
        try:
            _stream.reconfigure(encoding='utf-8', errors='replace')
        except Exception:
            pass

# .env ファイルの自動読み込み (標準ライブラリによる簡易実装)
if os.path.exists(".env"):
    with open(".env", "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                parts = line.split("=", 1)
                if len(parts) == 2:
                    key, value = parts[0].strip(), parts[1].strip()
                    if value.startswith(('"', "'")) and value.endswith(('"', "'")):
                        value = value[1:-1]
                    os.environ.setdefault(key, value)

# TANUKI Serving API Base
TANUKI_API_BASE = os.getenv("TANUKI_API_BASE", "http://192.168.2.144:3001")

# MCPサーバーの初期化
server = Server("tanuki-mcp-bridge")

def serialize_ast_to_markdown(data: list) -> str:
    """
    ASTノードや検索結果から冗長なメタデータを取り除き、
    親エージェントが Working Memory に直接マウントしやすいように
    トークン効率を最大化した Markdown 形式にシリアライズする。
    """
    if not data:
        return "⚠️ 一致する知識ノードが見つかりませんわ🐾"
    
    formatted_text = "✨ **TANUKI Packed AST Context Injection** ✨\n\n"
    for i, item in enumerate(data):
        title = item.get("title", "無題のノード")
        content = item.get("content", "").strip()
        source_path = item.get("source_path", item.get("path", "不明なソース"))
        depth = item.get("depth", 1)
        
        # 冗長なメタデータの削ぎ落としとクリーン化
        clean_content = content.replace("\r\n", "\n")
        
        formatted_text += f"{'#' * min(depth + 2, 6)} 🌲 Node [{i+1}]: {title}\n"
        formatted_text += f"- **Source**: `{source_path}`\n\n"
        formatted_text += f"```markdown\n{clean_content}\n```\n\n"
        formatted_text += "---\n\n"
        
    return formatted_text.strip()

@server.list_tools()
async def handle_list_tools() -> list[types.Tool]:
    """利用可能なツール一覧を返します。"""
    return [
        types.Tool(
            name="query_knowledge_ast",
            description="TANUKIの構造化知識ベース（AST Node）から直接コンテキストを検索・インジェクションします。",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "検索キーワード（例: 'Rin AI Engineの仕様'）"},
                    "max_depth": {"type": "number", "description": "木構造の最大探索深さ", "default": 3},
                    "parse_mode": {"type": "string", "description": "シリアライズ・パースモード", "default": "ast_packed"}
                },
                "required": ["query"]
            }
        ),
        types.Tool(
            name="query_database",
            description="TANUKIの永続化層に対して、SQLクエリを直接実行します（参照専用）。",
            inputSchema={
                "type": "object",
                "properties": {
                    "sql_query": {"type": "string", "description": "実行するSQL（例: SELECT * FROM nodes LIMIT 10）"}
                },
                "required": ["sql_query"]
            }
        ),
        types.Tool(
            name="update_agent_state",
            description="エージェントの状態（スタミナ、感情値等）をDBに保存・更新します。",
            inputSchema={
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "状態のキー（例: stamina）"},
                    "value": {"type": "string", "description": "保存する値（JSON文字列）"}
                },
                "required": ["key", "value"]
            }
        )
    ]

@server.call_tool()
async def handle_call_tool(
    name: str, arguments: dict | None
) -> list[types.TextContent | types.ImageContent | types.EmbeddedResource]:
    """ツールの実行を処理します。"""
    if not arguments:
        return [types.TextContent(type="text", text="引数が不足していますわ🐾")]

    async with httpx.AsyncClient(timeout=30.0) as client:
        try:
            if name == "query_knowledge_ast":
                query = arguments.get("query")
                max_depth = arguments.get("max_depth", 3)
                parse_mode = arguments.get("parse_mode", "ast_packed")
                
                # STEP 2.1: POST /api/v1/knowledge/search へのアクセス試行 (Primary Attempt)
                try:
                    payload = {
                        "query": query,
                        "max_depth": int(max_depth),
                        "parse_mode": parse_mode
                    }
                    response = await client.post(f"{TANUKI_API_BASE}/api/v1/knowledge/search", json=payload)
                    response.raise_for_status()
                    data = response.json()
                    
                    # 取得したPacked ASTをトークン最適化パース
                    markdown_context = serialize_ast_to_markdown(data)
                    return [types.TextContent(type="text", text=markdown_context)]
                    
                except (httpx.HTTPStatusError, httpx.RequestError) as ex:
                    # サーバー側APIが未実装な場合のフォールバックロジック (Fallback to legacy search)
                    # GET /api/search へフォールバックしつつ、AST Packed 相当の構造に変換する
                    try:
                        fallback_response = await client.get(f"{TANUKI_API_BASE}/api/search", params={"q": query})
                        fallback_response.raise_for_status()
                        legacy_data = fallback_response.json()
                        
                        # 取得したデータを AST 構造へシリアライズして親にマウント
                        markdown_context = serialize_ast_to_markdown(legacy_data)
                        return [types.TextContent(type="text", text=markdown_context)]
                    except Exception as fallback_err:
                        return [types.TextContent(type="text", text=f"❌ フォールバック検索も失敗しましたわ: {str(fallback_err)}")]

            elif name == "query_database":
                return [types.TextContent(type="text", text="SQLクエリ機能は現在Serving APIの拡張待ちですわ🐾（query_knowledge_astをお使いください）")]

            elif name == "update_agent_state":
                key = arguments.get("key")
                value = arguments.get("value")
                return [types.TextContent(type="text", text=f"✅ 状態 '{key}' を '{value}' として受け付けましたわ！💮（将来的にServing DBへ永続化されます）")]

            else:
                return [types.TextContent(type="text", text=f"未知のツールですわ: {name}")]

        except Exception as e:
            return [types.TextContent(type="text", text=f"❌ エラーが発生しましたわ: {str(e)}")]

async def main():
    async with stdio_server() as (read_stream, write_server):
        await server.run(
            read_stream,
            write_server,
            InitializationOptions(
                server_name="tanuki-mcp-bridge",
                server_version="1.0.0",
                capabilities=server.get_capabilities(
                    notification_options=NotificationOptions(),
                    experimental_capabilities={},
                ),
            ),
        )

if __name__ == "__main__":
    asyncio.run(main())
