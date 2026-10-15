param(
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$Owner = 'chxchx-dev'
$Repository = 'chxchx-tech-lead'
$PreviewTag = 'studio-dev-preview'
$Base = "https://github.com/$Owner/$Repository/releases/download/$PreviewTag"
$Platform = 'windows-x86_64'
$ExpectedPackage = 'chxchx-studio-windows-x86_64.zip'

if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw 'El preview actual requiere Windows x86_64.'
}

$AppParent = Join-Path $env:LOCALAPPDATA 'Programs\ChxChxStudio'
$Launcher = Join-Path $AppParent 'chxchx-studio-dev.cmd'
$StartMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\ChxChx Studio Dev.lnk'
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "chxchx-studio-$PID"
$Stage = $null
New-Item -ItemType Directory -Path $TempRoot | Out-Null
try {
    $ManifestPath = Join-Path $TempRoot 'studio-preview.txt'
    $CacheBuster = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/studio-preview.txt?t=$CacheBuster" -OutFile $ManifestPath
    $Manifest = @{}
    foreach ($Line in Get-Content -LiteralPath $ManifestPath) {
        if ($Line -match '^([a-z0-9.-]+)=(.*)$') {
            if ($Manifest.ContainsKey($Matches[1])) { throw "Clave repetida en el manifiesto: $($Matches[1])" }
            $Manifest[$Matches[1]] = $Matches[2]
        }
    }
    if ($Manifest['schema_version'] -ne '1') { throw 'Versión de manifiesto no compatible.' }
    $Commit = $Manifest['commit']
    if ($Commit -notmatch '^[0-9a-f]{40}$') { throw 'El manifiesto contiene un commit inválido.' }
    if ($Manifest['version'] -notmatch '^[0-9A-Za-z.+-]+$') { throw 'El manifiesto contiene una versión inválida.' }
    if ($Manifest["asset.$Platform"] -ne $ExpectedPackage) { throw 'El manifiesto apunta a un paquete inesperado.' }
    $WheelName = $Manifest['asset.cli-wheel']
    if ($WheelName -notmatch '^chxchx_tech_lead-[A-Za-z0-9.+!_-]+-py3-none-any\.whl$') { throw 'El manifiesto apunta a un wheel inesperado.' }
    foreach ($Key in @("sha256.$Platform", 'sha256.cli-wheel')) {
        if ($Manifest[$Key] -notmatch '^[0-9a-f]{64}$') { throw "El manifiesto contiene un checksum inválido: $Key" }
    }

    $ShortCommit = $Commit.Substring(0, 12)
    $AppRoot = Join-Path $AppParent "dev-$ShortCommit"
    $PackagePath = Join-Path $TempRoot $ExpectedPackage
    $WheelPath = Join-Path $TempRoot $WheelName
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/$ExpectedPackage?t=$CacheBuster" -OutFile $PackagePath
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/$WheelName?t=$CacheBuster" -OutFile $WheelPath
    $PackageHash = (Get-FileHash -LiteralPath $PackagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $WheelHash = (Get-FileHash -LiteralPath $WheelPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($PackageHash -ne $Manifest["sha256.$Platform"]) { throw 'El SHA-256 de Studio no coincide.' }
    if ($WheelHash -ne $Manifest['sha256.cli-wheel']) { throw 'El SHA-256 del CLI no coincide.' }

    if ($DryRun) {
        Write-Host "DRY RUN: las descargas y checksums son válidos. Se instalaría $AppRoot."
        Write-Host 'No se modificaron archivos de instalación ni se instaló uv.'
        return
    }
    if ((Test-Path -LiteralPath $AppRoot) -and -not (Test-Path -LiteralPath (Join-Path $AppRoot '.chxchx-managed'))) {
        throw "La ruta $AppRoot ya existe y no está administrada por ChxChx. No la modifiqué."
    }
    if (Test-Path -LiteralPath $StartMenu) {
        $ExistingShortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($StartMenu)
        if ($ExistingShortcut.TargetPath -ne $Launcher) { throw "Ya existe un acceso no administrado: $StartMenu" }
    }
    if ((Test-Path -LiteralPath $Launcher) -and -not ((Get-Content -LiteralPath $Launcher -Raw) -match 'CHXCHX_STUDIO_DEV_LAUNCHER')) {
        throw "Ya existe $Launcher y no está administrado por ChxChx. No lo sobrescribí."
    }

    New-Item -ItemType Directory -Path $AppParent -Force | Out-Null
    if (-not (Test-Path -LiteralPath $AppRoot)) {
        $Stage = Join-Path $AppParent ".chxchx-stage-$ShortCommit-$PID"
        if (Test-Path -LiteralPath $Stage) { Remove-Item -LiteralPath $Stage -Recurse -Force }
        New-Item -ItemType Directory -Path $Stage | Out-Null
        Expand-Archive -LiteralPath $PackagePath -DestinationPath $Stage
        $Binary = Get-ChildItem -LiteralPath $Stage -Filter 'chxchx-studio.exe' -File -Recurse | Select-Object -First 1
        if ($null -eq $Binary) { throw 'El ZIP no contiene chxchx-studio.exe.' }
        Set-Content -LiteralPath (Join-Path $Stage '.chxchx-managed') -Value $Commit -NoNewline
        Move-Item -LiteralPath $Stage -Destination $AppRoot
        $Stage = $null
    } elseif ($null -eq (Get-ChildItem -LiteralPath $AppRoot -Filter 'chxchx-studio.exe' -File -Recurse | Select-Object -First 1)) {
        throw "La instalación administrada está incompleta: falta chxchx-studio.exe en $AppRoot."
    }

    $Uv = Get-Command uv -ErrorAction SilentlyContinue
    if ($null -eq $Uv) {
        $UvCandidate = Join-Path $HOME '.local\bin\uv.exe'
        if (-not (Test-Path -LiteralPath $UvCandidate)) {
            irm https://astral.sh/uv/install.ps1 | iex
        }
        $UvCandidate = Join-Path $HOME '.local\bin\uv.exe'
        if (-not (Test-Path -LiteralPath $UvCandidate)) { throw 'No se encontró uv después de instalarlo.' }
        $UvPath = $UvCandidate
    } else {
        $UvPath = $Uv.Source
    }

    $PythonEnvironment = Join-Path $AppRoot '.venv'
    $Python = Join-Path $PythonEnvironment 'Scripts\python.exe'
    & $UvPath venv --python 3.11 $PythonEnvironment
    & $UvPath pip install --python $Python $WheelPath
    if (-not (Test-Path -LiteralPath (Join-Path $PythonEnvironment 'Scripts\chxchx-tech.exe'))) {
        throw 'El wheel no dejó el comando chxchx-tech esperado.'
    }

    $LauncherTemp = "$Launcher.tmp-$PID"
    @"
@echo off
rem CHXCHX_STUDIO_DEV_LAUNCHER
set "APP_ROOT=%LOCALAPPDATA%\Programs\ChxChxStudio\dev-$ShortCommit"
set "CHXCHX_TECH_CLI=%APP_ROOT%\.venv\Scripts\chxchx-tech.exe"
"%APP_ROOT%\bin\chxchx-studio.exe" %*
"@ | Set-Content -LiteralPath $LauncherTemp -Encoding Ascii
    Move-Item -LiteralPath $LauncherTemp -Destination $Launcher -Force

    New-Item -ItemType Directory -Path (Split-Path -Parent $StartMenu) -Force | Out-Null
    $Shell = New-Object -ComObject WScript.Shell
    $Shortcut = $Shell.CreateShortcut($StartMenu)
    $Shortcut.TargetPath = $Launcher
    $Shortcut.WorkingDirectory = $AppRoot
    $Shortcut.Description = "ChxChx Studio preview $ShortCommit"
    $Shortcut.Save()

    Write-Host "Studio quedó instalado desde el preview $ShortCommit ($($Manifest['version']))."
    Write-Host "Abre desde Inicio o ejecuta el archivo: $Launcher"
} finally {
    if ($null -ne $Stage -and (Test-Path -LiteralPath $Stage)) {
        Remove-Item -LiteralPath $Stage -Recurse -Force
    }
    if (Test-Path -LiteralPath $TempRoot) { Remove-Item -LiteralPath $TempRoot -Recurse -Force }
}
