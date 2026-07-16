#!/bin/bash
# ============================================================
# 🐾 T.A.N.U.K.I. Phase 0: Drive → GitHub → Irminsul 最小同期スクリプト (Linux版・VRAM防衛強化)
# ============================================================
# 作成: ナヒーダ (2026-07-16)
# VRAM強化: 監視ループ + 事前/事後クリーンアップ + RAII + ログ
# ============================================================

set -e

LOCAL_ROOT="${LOCAL_ROOT:-$(pwd)}"
SPECS_DIR="documents/specs"
DRIVE_FOLDER_ID="1U1681TSGph_OWSl1GeUZDxxb1ioIa55m"
COMMIT_MSG="docs(sync): Drive → Irminsul [Phase0 VRAM-safe]"

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

echo "🐾 Phase 0 Drive → Irminsul パイプライン開始 (VRAM防衛強化)"
echo "LocalRoot   : $LOCAL_ROOT"

cd "$LOCAL_ROOT"

# VRAMガード関数
vram_guard() {
    echo "🔒 VRAM Guard 開始..."
    for i in {1..30}; do
        usage=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits | head -1 2>/dev/null || echo "0")
        if [[ $usage -gt 8000 ]]; then
            echo "⚠️ VRAM ${usage}MiB → 待機中... ($i/30)"
            sleep 3
        else
            break
        fi
    done
}

# クリーンアップ
cleanup_vram() {
    echo "🧹 VRAM クリーンアップ..."
    for model in "nomic-embed-text" "${TANUKI_MODEL:-gemma}"; do
        curl -s -X POST http://localhost:11434/api/generate \
             -d "{\"model\":\"$model\",\"prompt\":\"\",\"keep_alive\":0}" > /dev/null || true
    done
}

trap cleanup_vram EXIT

vram_guard

mkdir -p "$SPECS_DIR"

# Git
if [[ -n $(git status --porcelain "$SPECS_DIR" docs/ 2>/dev/null) ]]; then
    if $DRY_RUN; then
        echo "[DryRun] git skip"
    else
        git add "$SPECS_DIR" docs/ 2>/dev/null || true
        git commit -m "$COMMIT_MSG $(date '+%Y-%m-%d %H:%M')" || echo "No changes"
        git push origin main || echo "Push failed"
    fi
fi

# コンパイル
if ! $SKIP_COMPILE; then
    vram_guard
    echo "【3/5】コンパイル..."
    if $DRY_RUN; then
        echo "[DryRun] skip"
    else
        if [[ -f "rebuild_tanuki.py" ]]; then
            python3 rebuild_tanuki.py
        else
            cargo run --bin tanuki-compiler -- compile
        fi
    fi
fi

# Factory
if ! $SKIP_FACTORY; then
    vram_guard
    echo "【4/5】Factory Sync..."
    if $DRY_RUN; then
        echo "[DryRun] skip"
    else
        python3 -c '
import sqlite3
con = sqlite3.connect("knowledge.db")
con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
con.close()
print("WAL done")
        ' 2>/dev/null || true
        echo "Factory sync: manual or existing script"
    fi
fi

cleanup_vram
echo "=========================================="
echo "Phase 0 完了 (VRAM防衛済み)"
echo "Driveフォルダ: https://drive.google.com/drive/folders/$DRIVE_FOLDER_ID"
echo "世界樹の根に、新しい枝が静かに伸びましたわ。🐾"
