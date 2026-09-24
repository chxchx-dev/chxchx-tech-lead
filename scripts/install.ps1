$ErrorActionPreference = "Stop"

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "uv no está instalado. Instalando..."
    irm https://astral.sh/uv/install.ps1 | iex
    $env:Path = "$env:USERPROFILE\.local\bin;$env:Path"
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
uv tool install --force $Root

Write-Host "Instalado. Ejecuta: chichan doctor"
