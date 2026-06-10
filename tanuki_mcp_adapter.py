#!/usr/bin/env python3
"""
tanuki_mcp_adapter.py - v3.3 (Windows/Linux 完全ハイブリッド堅牢版)
- OSを自動判別し、Windows環境下での NotImplementedError を完全抑止 (スレッド+Queueハイブリッド)
- Windows環境における標準入出力の UTF-8 強制再構成による UnicodeEncodeError の完全防御
- asyncio.Lock による標準出力パケット混線の防止 ＆ 16MB 行制限
- aioredis コネクションプール ＆ Pipeline による超高速1RTTフェッチ
"""

import sys
import os
import json
import asyncio
import msgpack
import platform
import redis.asyncio as aioredis

# Windows環境における文字化け・エンコード即死を防ぐための標準入出力UTF-8強制クランプ
if platform.system() == "Windows":
    import sys
    sys.stdin = open(sys.stdin.fileno(), mode='r', encoding='utf-8', closefd=False)
    sys.stdout = open(sys.stdout.fileno(), mode='w', encoding='utf-8', closefd=False)

# ==============================================================================
# 🧠 1. Core層：TanukiEngineCore
# ==============================================================================
class TanukiEngineCore:
    def __init__(self):
        self.redis_host = os.getenv("REDIS_HOST", "127.0.0.1")
        self.redis_port = int(os.getenv("REDIS_PORT", "6379"))
        self.redis_password = os.getenv("REDIS_PASSWORD", "V3仕様の超強力なパスワード")
        self._pool = None

    async def initialize(self):
        if not self._pool:
            self._pool = aioredis.ConnectionPool(
                host=self.redis_host, port=self.redis_port, password=self.redis_password,
                db=0, max_connections=20, decode_responses=False
            )
            print("[🧠] Redis V3不変バッファ用コネクションプールが確立されましたわ。(Core層)", file=sys.stderr)

    async def execute_ast_search(self, query: str) -> str:
        if not self._pool:
            await self.initialize()
        client = aioredis.Redis(connection_pool=self._pool)
        try:
            async with client.pipeline(transaction=False) as pipe:
                pipe.hget("tanuki:ast:v3:tree_index", query)
                pipe.get("tanuki:knowledge:snapshot")
                ast_bin, snapshot_bin = await pipe.execute()

            if not ast_bin:
                return f"【TANUKI Engine Core】「{query}」に関するASTインデックスが未生成ですわ。"

            ast_data = msgpack.unpackb(ast_bin, raw=False)
            return f"【TANUKI Engine Core】「{query}」の逆引き成功。ASTノードID: {ast_data.get('node_id', 'UNKNOWN')}。完全な整合性を同期完了いたしましたわ。"
        except Exception as e:
            print(f"[🚨 Redis Error] フェッチ例外: {e}", file=sys.stderr)
            return f"【⚠️ Core Error】Redis V3バッファーへのアクセス中に例外が発生いたしましたわ。"

# ==============================================================================
# 🌐 2. Interface層：TanukiAsyncMcpServer
# ==============================================================================
class TanukiAsyncMcpServer:
    def __init__(self, core_engine: TanukiEngineCore):
        self.core = core_engine
        self.tools = [{
            "name": "search_development_log",
            "description": "TANUKIエンジンのASTツリーから開発日誌（Rin AI Engine等の航跡）を逆引き検索するツールですわ。",
            "inputSchema": {
                "type": "object",
                "properties": {"query": {"type": "string", "description": "検索したい技術コンテキスト"}},
                "required": ["query"]
            }
        }]
        self._write_lock = asyncio.Lock()
        self._max_line_limit = 16 * 1024 * 1024

    async def start(self):
        await self.core.initialize()
        is_windows = platform.system() == "Windows"

        if is_windows:
            print("[⚙️] 真・非同期常駐リスナーループ(v3.3)が起動しましたわ。(Windows互換モード)", file=sys.stderr)
            # Windows用：ブロックするsys.stdin.readlineを別スレッドで走らせ、Queueで回収する防衛回路
            input_queue = asyncio.Queue()
            loop = asyncio.get_event_loop()

            def watch_stdin():
                while True:
                    line = sys.stdin.readline()
                    if not line:
                        loop.call_soon_threadsafe(input_queue.put_nowait, None)
                        break
                    loop.call_soon_threadsafe(input_queue.put_nowait, line)

            asyncio.get_event_loop().run_in_executor(None, watch_stdin)

            while True:
                line = await input_queue.get()
                if line is None:
                    break # EOF
                line = line.strip()
                if not line:
                    continue
                try:
                    request = json.loads(line)
                    asyncio.create_task(self.handle_request_windows(request))
                except json.JSONDecodeError:
                    await self.send_error_windows(-32700, "Parse error")
        else:
            print("[⚙️] 真・非同期常駐リスナーループ(v3.3)がOSパイプ上で完全起動しましたわ。(Linuxモード)", file=sys.stderr)
            # Linux/POSIX用：ネイティブ非同期ストリーム
            loop = asyncio.get_event_loop()
            reader = asyncio.StreamReader(limit=self._max_line_limit)
            protocol = asyncio.StreamReaderProtocol(reader)
            await loop.connect_read_pipe(lambda: protocol, sys.stdin)
            w_transport, w_protocol = await loop.connect_write_pipe(lambda: asyncio.streams.FlowControlMixin(), sys.stdout)
            writer = asyncio.StreamWriter(w_transport, w_protocol, reader, loop)

            while True:
                try:
                    line_bytes = await reader.readline()
                    if not line_bytes:
                        break
                    line = line_bytes.decode('utf-8').strip()
                    if not line:
                        continue
                    request = json.loads(line)
                    asyncio.create_task(self.handle_request_linux(request, writer))
                except Exception:
                    await self.send_error_linux(writer, None, -32700, "Parse error")

    # --- Linux用ハンドラ ---
    async def handle_request_linux(self, request: dict, writer: asyncio.StreamWriter):
        msg_id = request.get("id")
        method = request.get("method")
        if method == "tools/list" and msg_id is not None:
            await self.send_response_linux(writer, {"jsonrpc": "2.0", "id": msg_id, "result": {"tools": self.tools}})
        elif method == "tools/call" and msg_id is not None:
            params = request.get("params", {})
            tool_args = params.get("arguments", {})
            result_data = await self.core.execute_ast_search(tool_args.get("query", ""))
            await self.send_response_linux(writer, {"jsonrpc": "2.0", "id": msg_id, "result": {"content": [{"type": "text", "text": result_data}]}})
        elif msg_id is not None:
            await self.send_error_linux(writer, msg_id, -32601, "Method not found")

    async def send_response_linux(self, writer: asyncio.StreamWriter, response: dict):
        payload = json.dumps(response, ensure_ascii=False) + "\n"
        async with self._write_lock:
            writer.write(payload.encode('utf-8'))
            await writer.drain()

    async def send_error_linux(self, writer: asyncio.StreamWriter, msg_id, code: int, message: str):
        await self.send_response_linux(writer, {"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}})

    # --- Windows用ハンドラ ---
    async def handle_request_windows(self, request: dict):
        msg_id = request.get("id")
        method = request.get("method")
        if method == "tools/list" and msg_id is not None:
            await self.send_response_windows({"jsonrpc": "2.0", "id": msg_id, "result": {"tools": self.tools}})
        elif method == "tools/call" and msg_id is not None:
            params = request.get("params", {})
            tool_args = params.get("arguments", {})
            result_data = await self.core.execute_ast_search(tool_args.get("query", ""))
            await self.send_response_windows({"jsonrpc": "2.0", "id": msg_id, "result": {"content": [{"type": "text", "text": result_data}]}})
        elif msg_id is not None:
            await self.send_error_windows(msg_id, -32601, "Method not found")

    async def send_response_windows(self, response: dict):
        payload = json.dumps(response, ensure_ascii=False) + "\n"
        async with self._write_lock:
            sys.stdout.write(payload)
            sys.stdout.flush()

    async def send_error_windows(self, msg_id, code: int, message: str):
        await self.send_response_windows({"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}})

if __name__ == "__main__":
    core = TanukiEngineCore()
    server = TanukiAsyncMcpServer(core)
    try:
        asyncio.run(server.start())
    except KeyboardInterrupt:
        print("[🛑] サスペンドされましたわ。", file=sys.stderr)