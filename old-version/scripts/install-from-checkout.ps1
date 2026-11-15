$ErrorActionPreference = "Stop"

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "uv no está instalado. Instalando..."
    irm https://astral.sh/uv/install.ps1 | iex
    $env:Path = "$env:USERPROFILE\.local\bin;$env:Path"
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
uv tool install --force $Root

$ChxChxPath = $null
$ChxChxCommand = Get-Command chxchx-tech -ErrorAction SilentlyContinue
if ($null -ne $ChxChxCommand) {
    $ChxChxPath = $ChxChxCommand.Source
} else {
    $Candidate = Join-Path $env:USERPROFILE ".local\bin\chxchx-tech.exe"
    if (Test-Path $Candidate) {
        $ChxChxPath = $Candidate
    }
}
if ($null -ne $ChxChxPath) {
    Write-Host "Instalando herramientas gestionadas..."
    & $ChxChxPath install
} else {
    Write-Host "ChxChx quedó instalado, pero no está en PATH todavía. Ejecuta: chxchx-tech install"
}

Write-Host "Instalado. Ejecuta: chxchx-tech doctor"
