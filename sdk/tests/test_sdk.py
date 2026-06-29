import os
import sys
import time
import pytest
import asyncio
import subprocess
from pathlib import Path

# テストターゲットとなるモジュールへのパスを追加
sdk_dir = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(sdk_dir))

# ------------------------------------------------------------------------------
# 1. Rust 拡張 (tanuki_rust) のユニットテスト
# ------------------------------------------------------------------------------
def test_rust_extension_import():
    """Maturinでビルドされた Rust 拡張モジュールのインポートとハッシュ計算をテストします。"""
    try:
        import tanuki_rust
    except ImportError:
        pytest.skip("tanuki_rust extension is not built/installed. Skipping.")

    # FNV-1a ハッシュ計算の正常テスト
    hash_val = tanuki_rust.calculate_fnv1a("test")
    assert isinstance(hash_val, int)
    assert hash_val != 0


# ------------------------------------------------------------------------------
# 2. Python SDK (tanuki) のインポートテスト
# ------------------------------------------------------------------------------
def test_sdk_import():
    """Pure Python SDK のインポートを検証します。"""
    from tanuki import TanukiClient
    assert TanukiClient is not None


# ------------------------------------------------------------------------------
# 3. API サーバー結合テスト (pytest フィクスチャを利用)
# ------------------------------------------------------------------------------
@pytest.fixture(scope="module")
def tanuki_serving_process():
    """テスト用の tanuki-serving API サーバープロセスを起動・終了します。"""
    project_root = sdk_dir.parent
    
    # テスト用のダミーデータベースを指定
    test_db = project_root / "test_knowledge.db"
    if test_db.exists():
        test_db.unlink()

    # 開発環境のデバッグバイナリを優先探索、なければ cargo run
    serving_bin = project_root / "target" / "debug" / "tanuki-serving.exe"
    if not serving_bin.exists():
        # Linux / WSL 用バイナリパスフォールバック
        serving_bin = project_root / "target" / "debug" / "tanuki-serving"

    if serving_bin.exists():
        cmd = [str(serving_bin)]
    else:
        cmd = ["cargo", "run", "--bin", "tanuki-serving"]

    # テスト用環境変数の適用
    env = os.environ.copy()
    env["TANUKI_DB_PATH"] = str(test_db) # DBの分離
    
    print(f"\n[INFO] Starting test API server with: {cmd}")
    proc = subprocess.Popen(
        cmd,
        cwd=str(project_root),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    # サーバーの起動待機
    time.sleep(3.0)

    yield proc

    # テスト終了後にサーバーを確実にクリーンアップ
    print("\n[INFO] Stopping test API server...")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()

    # テスト用DBのクリーンアップ
    if test_db.exists():
        try:
            test_db.unlink()
        except OSError:
            pass


@pytest.mark.asyncio
async def test_api_endpoints(tanuki_serving_process):
    """起動したテストサーバーに対して、SDKクライアントを用いた疎通テストを実行します。"""
    from tanuki import TanukiClient

    # 3000ポート（デフォルト）で接続
    async with TanukiClient(base_url="http://localhost:3000") as client:
        # 1. ヘルスチェック
        health = await client.health()
        assert health is not None
        assert "online" in health

        # 2. ノード取得 (初期状態は空のはず)
        nodes = await client.get_nodes()
        assert isinstance(nodes, list)
        
        # 3. 検索疎通 (空のインデックスでも動作することを確認)
        results = await client.search("test")
        assert isinstance(results, list)
