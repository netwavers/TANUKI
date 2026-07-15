#!/bin/bash
# ============================================================
# 🐾 T.A.N.U.K.I. Phase 0: Drive → GitHub → Irminsul 最小同期スクリプト (Linux版)
# ============================================================
# 作成: ナヒーダ (2026-07-16)
# 目的: Google Drive の仕様書を決定論的に TANUKI 知識木へ根付かせる
#
# 使い方 (Linux / WSL):
#   cd /path/to/TANUKI
#   bash scripts/drive_to_tanuki.sh
#   bash scripts/drive_to_tanuki.sh --dry-run
#   bash scripts/drive_to_tanuki.sh --skip-compile
#   bash scripts/drive_to_tanuki.sh --skip-factory
#
# 前提:
#   1. Driveフォルダから .md を documents/specs/ に手動コピー
#   2. git, python3, cargo が利用可能
#   3. scripts/sync_kb_to_factory.ps1 相当のロジックをbashで実装
# ============================================================

set -e

LOCAL_ROOT="${LOCAL_ROOT:-$(pwd)}"
SPECS_DIR="documents/specs"
DRIVE_FOLDER_ID="1U1681TSGph_OWSl1GeUZDxxb1ioIa55m"
COMMIT_MSG="docs(sync): Drive → Irminsul [Phase0 auto]"

DRY_RUN=false
SKIP_COMPILE=false
SKIP_FACTORY=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run) DRY_RUN=true ;;
    --skip-compile) SKIP_COMPILE=true ;;
    --skip-factory) SKIP_FACTORY=true ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

echo "🐾 Phase 0 Drive → Irminsul パイプライン開始 (Linux)"
echo "LocalRoot   : $LOCAL_ROOT"
echo "SpecsDir    : $SPECS_DIR"
echo "DryRun      : $DRY_RUN"

cd "$LOCAL_ROOT"

# 1. specs ディレクトリ準備
mkdir -p "$SPECS_DIR"

# 2. Git コミット
if [[ -n $(git status --porcelain "$SPECS_DIR" docs/ 2>/dev/null) ]]; then
  if $DRY_RUN; then
    echo "[DryRun] git add & commit をスキップ"
  else
    git add "$SPECS_DIR" docs/ 2>/dev/null || true
    git commit -m "$COMMIT_MSG $(date '+%Y-%m-%d %H:%M')" || echo "No changes to commit"
    git push origin main || echo "Push failed (check manually)"
  fi
else
  echo "Git 上に新規変更はありません。"
fi

# 3. コンパイル
if $SKIP_COMPILE; then
  echo "コンパイルをスキップ (--skip-compile)"
else
  echo "【3/5】tanuki-compiler 実行..."
  if $DRY_RUN; then
    echo "[DryRun] python3 rebuild_tanuki.py をスキップ"
  else
    if [[ -f "rebuild_tanuki.py" ]]; then
      python3 rebuild_tanuki.py
    else
      cargo run --bin tanuki-compiler -- compile
    fi
  fi
fi

# 4. Factory Sync
if $SKIP_FACTORY; then
  echo "Factory Sync をスキップ (--skip-factory)"
else
  echo "【4/5】Factory Sync 実行..."
  if $DRY_RUN; then
    echo "[DryRun] sync をスキップ"
  else
    # Linux用に簡易版を実装（既存 ps1 のロジックを参考）
    echo "WAL checkpoint..."
    python3 -c '
import sqlite3
con = sqlite3.connect("knowledge.db")
con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
con.close()
print("WAL checkpoint done")
    ' 2>/dev/null || true

    # 簡易 sync（本格版は後で拡張）
    echo "Factory sync は手動または既存スクリプトを使用してください。"
    echo "例: scp knowledge.db tanuki-factory:/home/tanuki/RinAISystem/TANUKI/"
  fi
fi

echo "========================================"
echo "Phase 0 完了"
echo "次にやること:"
echo "  1. Drive から .md を documents/specs/ にコピー"
echo "  2. このスクリプトを再実行"
echo "  3. MCP で query_knowledge_ast(\"AGENT_IRMINSUL_NAV\") を確認"
echo "Driveフォルダ: https://drive.google.com/drive/folders/$DRIVE_FOLDER_ID"
echo "========================================"
echo "世界樹の根に、新しい枝が静かに伸びましたわ。🐾"
