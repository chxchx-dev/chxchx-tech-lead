$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

uv --cache-dir .uv-cache run --offline --no-sync chxchx-tech tui .
exit $LASTEXITCODE
