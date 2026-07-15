# ============================================================
# 🐾 T.A.N.U.K.I. Phase 0: Drive → GitHub → Irminsul 最小同期スクリプト
# ============================================================
# 作成: ナヒーダ (2026-07-16)
# 目的: Google Drive の仕様書フォルダを決定論的に TANUKI 知識木へ根付かせる
#
# 使い方 (PowerShell, リポジトリルートから):
#   .\scripts\drive_to_tanuki.ps1
#   .\scripts\drive_to_tanuki.ps1 -SkipCompile
#   .\scripts\drive_to_tanuki.ps1 -SkipFactorySync
#   .\scripts\drive_to_tanuki.ps1 -DryRun
#
# 前提:
#   1. Google Drive フォルダ "T.A.N.U.K.I. Specs & Agent Instructions"
#      ID: 1U1681TSGph_OWSl1GeUZDxxb1ioIa55m
#   2. ファイルは既に Drive からローカルにエクスポート済み、または
#      本スクリプト内で手動配置されたものを使用
#   3. git / python / cargo が PATH にあること
#   4. 既存の scripts/sync_kb_to_factory.ps1 が利用可能
# ============================================================

param(
    [string]$LocalRoot        = "D:\Projects\PyProjects\TANUKI",
    [string]$DriveFolderId    = "1U1681TSGph_OWSl1GeUZDxxb1ioIa55m",
    [string]$SpecsDir         = "documents\specs",
    [string]$CommitMessage    = "docs(sync): Drive → Irminsul [Phase0 auto]",
    [switch]$SkipCompile,
    [switch]$SkipFactorySync,
    [switch]$DryRun,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-TanukiLog {
    param([string]$Message, [string]$Level = "INFO")
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARN"  { "Yellow" }
        "OK"    { "Green" }
        default { "Cyan" }
    }
    Write-Host "[$ts] [$Level] $Message" -ForegroundColor $color
}

Write-TanukiLog "🐾 Phase 0 Drive → Irminsul パイプライン開始" "OK"
Write-TanukiLog "LocalRoot      : $LocalRoot"
Write-TanukiLog "DriveFolderId  : $DriveFolderId"
Write-TanukiLog "SpecsDir       : $SpecsDir"
Write-TanukiLog "DryRun         : $DryRun"

# ------------------------------------------------------------
# 1. 作業ディレクトリ確認
# ------------------------------------------------------------
if (-not (Test-Path $LocalRoot)) {
    throw "LocalRoot が見つかりません: $LocalRoot"
}
Set-Location $LocalRoot

$specsPath = Join-Path $LocalRoot $SpecsDir
if (-not (Test-Path $specsPath)) {
    Write-TanukiLog "specs ディレクトリを作成します: $specsPath"
    if (-not $DryRun) {
        New-Item -ItemType Directory -Path $specsPath -Force | Out-Null
    }
}

# ------------------------------------------------------------
# 2. ステージング（Phase 0 ではローカル artifacts または手動配置を想定）
#    将来 Phase 1 で Drive API 直接 export に置き換え
# ------------------------------------------------------------
Write-TanukiLog "【1/5】ステージング準備..."

# ここに Drive から export した .md を配置する想定
# 現在は documents/specs/ に既に存在するファイルを対象にする
$mdFiles = Get-ChildItem -Path $specsPath -Filter "*.md" -ErrorAction SilentlyContinue
if ($mdFiles.Count -eq 0) {
    Write-TanukiLog "警告: $specsPath に .md ファイルがありません。Drive から手動 export して配置してください。" "WARN"
    Write-TanukiLog "推奨: Google Drive フォルダからダウンロードしたファイルを $specsPath にコピー"
}

# ------------------------------------------------------------
# 3. Git 状態確認 & コミット
# ------------------------------------------------------------
Write-TanukiLog "【2/5】Git 状態確認..."

$gitStatus = git status --porcelain 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "git が利用できません。リポジトリルートで実行してください。"
}

if ($gitStatus) {
    Write-TanukiLog "変更が検出されました。"
    if ($DryRun) {
        Write-TanukiLog "[DryRun] git add $SpecsDir をスキップ"
    } else {
        git add $SpecsDir
        git add docs/ 2>$null   # 既存 docs も念のため
        $statusAfter = git status --porcelain
        if ($statusAfter) {
            $fullMsg = "$CommitMessage $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
            git commit -m $fullMsg
            Write-TanukiLog "コミット完了: $fullMsg" "OK"
            
            Write-TanukiLog "main へ push します..."
            git push origin main
            if ($LASTEXITCODE -ne 0) {
                Write-TanukiLog "push に失敗しました。手動で確認してください。" "ERROR"
            } else {
                Write-TanukiLog "push 成功" "OK"
            }
        } else {
            Write-TanukiLog "ステージする変更がありませんでした。" "WARN"
        }
    }
} else {
    Write-TanukiLog "Git 上に新規変更はありません。スキップします。" "WARN"
}

# ------------------------------------------------------------
# 4. TANUKI コンパイル（差分ビルド）
# ------------------------------------------------------------
if ($SkipCompile) {
    Write-TanukiLog "【3/5】コンパイルをスキップ (-SkipCompile)" "WARN"
} else {
    Write-TanukiLog "【3/5】tanuki-compiler 差分コンパイルを実行..."
    if ($DryRun) {
        Write-TanukiLog "[DryRun] python rebuild_tanuki.py をスキップ"
    } else {
        # 優先: rebuild_tanuki.py（存在すれば）
        $rebuildScript = Join-Path $LocalRoot "rebuild_tanuki.py"
        if (Test-Path $rebuildScript) {
            python $rebuildScript
        } else {
            # フォールバック: cargo 直接
            Write-TanukiLog "rebuild_tanuki.py が見つからないため cargo で実行"
            cargo run --bin tanuki-compiler -- compile
        }
        if ($LASTEXITCODE -ne 0) {
            Write-TanukiLog "コンパイルに失敗しました。知識ベースが更新されていない可能性があります。" "ERROR"
            if (-not $Force) { throw "Compile failed" }
        } else {
            Write-TanukiLog "コンパイル完了" "OK"
        }
    }
}

# ------------------------------------------------------------
# 5. Factory Sync（MCP / Serving 反映）
# ------------------------------------------------------------
if ($SkipFactorySync) {
    Write-TanukiLog "【4/5】Factory Sync をスキップ (-SkipFactorySync)" "WARN"
} else {
    Write-TanukiLog "【4/5】Factory Sync を実行..."
    $syncScript = Join-Path $LocalRoot "scripts\sync_kb_to_factory.ps1"
    if (Test-Path $syncScript) {
        if ($DryRun) {
            Write-TanukiLog "[DryRun] $syncScript をスキップ"
        } else {
            & $syncScript
            if ($LASTEXITCODE -ne 0) {
                Write-TanukiLog "Factory Sync に失敗しました。" "ERROR"
            } else {
                Write-TanukiLog "Factory Sync 完了。MCP が最新仕様を参照可能になりました。" "OK"
            }
        }
    } else {
        Write-TanukiLog "sync_kb_to_factory.ps1 が見つかりません。手動で同期してください。" "WARN"
    }
}

# ------------------------------------------------------------
# 6. 完了通知
# ------------------------------------------------------------
Write-TanukiLog "【5/5】Phase 0 パイプライン完了" "OK"
Write-TanukiLog "----------------------------------------------"
Write-TanukiLog "次にやること:"
Write-TanukiLog "  1. Drive フォルダから最新 .md を $specsPath にコピー"
Write-TanukiLog "  2. 本スクリプトを再実行"
Write-TanukiLog "  3. MCP で query_knowledge_ast(\"AGENT_IRMINSUL_NAV\") を試す"
Write-TanukiLog "Drive フォルダ: https://drive.google.com/drive/folders/$DriveFolderId"
Write-TanukiLog "----------------------------------------------"
Write-TanukiLog "世界樹の根に、新しい枝が静かに伸びましたわ。🐾" "OK"
