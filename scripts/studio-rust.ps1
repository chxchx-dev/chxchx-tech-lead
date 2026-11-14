<#
.SYNOPSIS
    Compila y abre el ChxChx Studio de Rust desde este checkout (Windows).
.DESCRIPTION
    El Studio Qt heredado instala un lanzador con el mismo nombre (`chxchx-studio`); este
    script ejecuta el binario de Rust por su ruta explícita para que nunca se mezclen.
    Studio de Rust no abre consola: si no arranca, el motivo queda en
    `%USERPROFILE%\.chxchx-tech-lead\logs\studio-startup.log`.
.PARAMETER Project
    Proyecto que se abre. Por defecto, la carpeta actual.
.PARAMETER Debug
    Usa el perfil de depuración en lugar del de publicación.
#>
param(
    [string]$Project = (Get-Location).Path,
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'migration\rust\Cargo.toml'
$profileName = if ($Debug) { 'debug' } else { 'release' }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'No se encontró cargo. Instala Rust desde https://rustup.rs y abre una terminal nueva.'
}

$cargoArgs = @('build', '--manifest-path', $manifest, '--bin', 'chxchx-studio')
if (-not $Debug) { $cargoArgs += '--release' }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { throw "cargo build falló (código $LASTEXITCODE)." }

$binary = Join-Path $root "migration\rust\target\$profileName\chxchx-studio.exe"
if (-not (Test-Path $binary)) { throw "No se encontró $binary tras compilar." }

Write-Host "Abriendo $binary"
$installed = Get-Command chxchx-studio -ErrorAction SilentlyContinue
if ($installed) {
    Write-Host "Instalado en el PATH (puede ser el Studio Qt): $($installed.Source)"
} else {
    Write-Host 'Instalado en el PATH: ninguno'
}

# Studio no abre consola (subsistema Windows): se lanza aparte y esta terminal queda libre.
Start-Process -FilePath $binary -ArgumentList ('"{0}"' -f (Resolve-Path $Project).Path)
