# TANUKI MCP Bridge 起動スクリプト (PowerShell)
# mcp_config.json 準拠版（恒常利用用）

$ErrorActionPreference = "Stop"

Write-Host "🐾 TANUKI MCP Bridge を起動します..." -ForegroundColor Cyan

# ディレクトリ移動
Set-Location "D:\Projects\PyProjects\TANUKI"

# 環境変数設定（mcp_config.json と完全一致）
$env:TANUKI_API_BASE = "http://192.168.2.144:3001"

# プロジェクトのvenv Pythonを使用（推奨）
$python = "D:\Projects\PyProjects\.venv\Scripts\python.exe"

Write-Host "TANUKI_API_BASE = $env:TANUKI_API_BASE" -ForegroundColor Yellow
Write-Host "Python: $python" -ForegroundColor Yellow
Write-Host "MCPサーバーを起動中... (Ctrl+C で停止)" -ForegroundColor Green

# MCP Bridge 起動（mcp_config.json と同じ設定）
& $python "D:\Projects\PyProjects\TANUKI\tanuki_mcp_bridge.py"

Write-Host "MCP Bridge が終了しました。" -ForegroundColor Red
