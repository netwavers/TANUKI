#!/usr/bin/env python3
import os
import sys
import subprocess

# ==============================================================================
# 🐾 T.A.N.U.K.I. Unified Test Runner (run_tests.py)
# ==============================================================================
#
# このスクリプトは、Rustコア、Python SDK、およびMaturin拡張のすべてのテストを
# 一括で実行し、OSS化の再現性と品質を自動検証しますわ🐾
# ==============================================================================

CLR_RESET = "\033[0m"
CLR_BOLD = "\033[1m"
CLR_GREEN = "\033[92m"
CLR_RED = "\033[91m"
CLR_YELLOW = "\033[93m"
CLR_CYAN = "\033[96m"

def log_section(title):
    print("\n" + "=" * 80)
    print(f"{CLR_BOLD}{CLR_CYAN}🐾 {title}{CLR_RESET}")
    print("=" * 80)

def run_command(cmd, cwd=None):
    print(f"👉 Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd)
    if result.returncode != 0:
        print(f"\n{CLR_RED}❌ Command failed with exit code: {result.returncode}{CLR_RESET}")
        sys.exit(result.returncode)
    return True

def check_python_packages():
    """必要なテストパッケージがインストールされているか確認し、無ければ導入します。"""
    required = ["pytest", "pytest-asyncio"]
    missing = []
    for pkg in required:
        try:
            __import__(pkg.replace("-", "_"))
        except ImportError:
            missing.append(pkg)
            
    if missing:
        print(f"{CLR_YELLOW}⚠️  Missing test dependencies: {missing}. Installing...{CLR_RESET}")
        run_command([sys.executable, "-m", "pip", "install"] + missing)

def main():
    # 0. 依存関係のチェックと導入
    log_section("Python Test Dependencies Validation")
    check_python_packages()
    
    # 1. Rust ワークスペーステストの実行
    log_section("Running Rust Unit Tests (cargo test)")
    # Rust コンポーネントをテスト
    run_command(["cargo", "test", "--workspace"])

    # 2. Python SDK のインストール検証 (開発モード)
    log_section("Installing Python SDK in development mode")
    run_command([sys.executable, "-m", "pip", "install", "-e", "./sdk"])

    # 3. Python 結合テストの実行 (pytest)
    log_section("Running Python SDK & Rust Extension Integration Tests (pytest)")
    run_command([sys.executable, "-m", "pytest", "sdk/tests", "-v", "-s"])

    log_section("All Tests Completed Successfully! 💮")
    print(f"{CLR_GREEN}{CLR_BOLD}🐾 ご主人様、すべてのテストスイート（Rust & Python）が正常に合格いたしましたわ！完璧です！{CLR_RESET}\n")

if __name__ == "__main__":
    # Windows環境での絵文字文字化け即死防止
    if sys.platform == "win32":
        import io
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8')
    main()
