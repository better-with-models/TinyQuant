$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    Write-Error "TinyQuant pre-commit: python is required but was not found."
}

& python .\scripts\verify_pre_commit.py
exit $LASTEXITCODE
