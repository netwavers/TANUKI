# Sync local TANUKI KB (after tanuki-compiler) to tanuki-factory Serving for MCP.
# Preferred CLI: tanuki-journal sync  (see factory_sync.py)
#
# Usage (PowerShell, from repo):
#   .\TANUKI\scripts\sync_kb_to_factory.ps1
# Optional:
#   .\TANUKI\scripts\sync_kb_to_factory.ps1 -SkipRestart

param(
    [string]$LocalRoot = "D:\Projects\PyProjects\TANUKI",
    [string]$RemoteHost = "tanuki-factory",
    [string]$RemoteRoot = "/home/tanuki/RinAISystem/TANUKI",
    [switch]$SkipRestart
)

$ErrorActionPreference = "Stop"

$db = Join-Path $LocalRoot "knowledge.db"
$out = Join-Path $LocalRoot "output_knowledge"
if (-not (Test-Path $db)) {
    throw "knowledge.db not found at $db — run compiler first (tanuki-journal write / rebuild_tanuki.py)."
}
if (-not (Test-Path $out)) {
    throw "output_knowledge not found at $out — compiler may have failed."
}

Write-Host "🐾 WAL checkpoint (TRUNCATE) on knowledge.db ..."
python -c "import sqlite3; con = sqlite3.connect(r'$db', timeout=120); con.execute('PRAGMA wal_checkpoint(TRUNCATE)'); con.close()"


$payload = Join-Path $LocalRoot "knowledge_sync_payload.tar.gz"
if (Test-Path $payload) { Remove-Item $payload -Force }

Write-Host "[1/4] Pack knowledge.db + output_knowledge ..."
Push-Location $LocalRoot
try {
    & tar -czf knowledge_sync_payload.tar.gz knowledge.db output_knowledge
} finally {
    Pop-Location
}

Write-Host "[2/4] Upload to ${RemoteHost}:${RemoteRoot} ..."
scp $payload "${RemoteHost}:${RemoteRoot}/"

Write-Host "[3/4] Extract on factory (backup old db) ..."
$remoteCmd = @"
set -e
cd '$RemoteRoot'
cp -a knowledge.db knowledge.db.bak.`$(date +%Y%m%d_%H%M%S) 2>/dev/null || true
tar -xzf knowledge_sync_payload.tar.gz
rm -f knowledge_sync_payload.tar.gz
rm -f knowledge.db-wal knowledge.db-shm
ls -la knowledge.db | head -1
"@
ssh $RemoteHost $remoteCmd

if (-not $SkipRestart) {
    Write-Host "[4/4] Restart tanuki-serving ..."
    ssh $RemoteHost "cd '$RemoteRoot' && docker compose restart tanuki-serving"
} else {
    Write-Host "[4/4] SkipRestart — restart tanuki-serving manually if needed."
}

Remove-Item $payload -Force -ErrorAction SilentlyContinue
Write-Host "Done. Verify: curl http://192.168.2.144:3001/api/search?q=YOUR_QUERY"