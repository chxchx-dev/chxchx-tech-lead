[CmdletBinding()]
param(
    [ValidateSet('stable', 'dev')]
    [string]$Channel = 'stable',
    [string]$Version,
    [string]$Commit,
    [string]$OutputDirectory = 'dist/windows-msi',
    [switch]$BuildSmokeUpdate
)

$ErrorActionPreference = 'Stop'
$Repository = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$CargoToml = Get-Content -LiteralPath (Join-Path $Repository 'Cargo.toml') -Raw
if (-not $Version) {
    $Match = [regex]::Match($CargoToml, '(?m)^version\s*=\s*"(?<version>[0-9]+\.[0-9]+\.[0-9]+)(?:-[^"]+)?"')
    if (-not $Match.Success) { throw 'No pude leer la versión de Cargo.toml.' }
    $Version = $Match.Groups['version'].Value
}
if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$') {
    throw "La versión del MSI debe ser X.Y.Z; recibí $Version."
}
if (-not $Commit) {
    $Commit = (& git -C $Repository rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'No pude leer el commit actual.' }
}
if ($Commit -notmatch '^[0-9a-f]{40}$') { throw 'El MSI requiere el SHA completo de 40 caracteres.' }
if ($Channel -eq 'stable' -and $CargoToml -match '(?m)^version\s*=\s*"[^"]*[-+][^"]*"') {
    throw 'Un MSI estable requiere una versión Cargo X.Y.Z sin sufijo.'
}

$Wix = Get-Command wix -ErrorAction SilentlyContinue
if ($null -eq $Wix) { throw 'No se encontró WiX 5.0.2 en PATH.' }
$Uv = Get-Command uv -ErrorAction SilentlyContinue
if ($null -eq $Uv) { throw 'No se encontró uv en PATH.' }
$Python = Get-Command python -ErrorAction SilentlyContinue
if ($null -eq $Python) { throw 'No se encontró Python para generar la fuente WiX.' }

$TemporaryParent = Join-Path $env:TEMP 'chxchx-windows-msi'
New-Item -ItemType Directory -Path $TemporaryParent -Force | Out-Null
$Work = Join-Path $TemporaryParent ([guid]::NewGuid().ToString('N'))
$WorkRoot = [System.IO.Path]::GetFullPath($Work)
$TemporaryRoot = [System.IO.Path]::GetFullPath($TemporaryParent).TrimEnd('\') + '\'
if (-not $WorkRoot.StartsWith($TemporaryRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "La carpeta de trabajo se salió de la raíz temporal autorizada: $WorkRoot"
}
$Success = $false

function Invoke-WixChecked {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$LogPath,
        [Parameter(Mandatory = $true)][string]$Action
    )
    & $Wix.Source @Arguments *> $LogPath
    $Code = $LASTEXITCODE
    if ($Code -ne 0) {
        Select-String -LiteralPath $LogPath -Pattern 'error WIX|error ICE' |
            Select-Object -Last 80 |
            ForEach-Object { Write-Host $_.Line }
        throw "Falló $Action (código $Code). Registro: $LogPath"
    }
    $Warnings = @(Select-String -LiteralPath $LogPath -Pattern 'warning WIX|warning ICE').Count
    if ($Warnings -gt 0) { Write-Host "Se completó $Action con $Warnings advertencias WiX." }
}

try {
    $Payload = Join-Path $Work 'payload'
    $RuntimeExtract = Join-Path $Work 'python-runtime-extract'
    $RuntimeDirectory = Join-Path $Payload '.venv\python'
    $ScriptsDirectory = Join-Path $Payload '.venv\Scripts'
    $SitePackages = Join-Path $RuntimeDirectory 'Lib\site-packages'
    $WheelDirectory = Join-Path $Work 'wheel'
    $Requirements = Join-Path $Work 'requirements.txt'
    New-Item -ItemType Directory -Path $Payload, $RuntimeExtract, $ScriptsDirectory, $WheelDirectory -Force | Out-Null

    # Pinned CPython 3.11 standalone x64 distribution; its digest is published in the
    # python-build-standalone 20261110 SHA256SUMS file.
    $PythonArchiveName = 'cpython-3.11.18+20261110-x86_64-pc-windows-msvc-install_only_stripped.tar.gz'
    $PythonArchiveUrl = 'https://github.com/astral-sh/python-build-standalone/releases/download/20261110/cpython-3.11.18%2B20261110-x86_64-pc-windows-msvc-install_only_stripped.tar.gz'
    $PythonArchiveSha256 = 'f31751f4a8483f5ac7cd77ad83f8dfb80d9f2a7245bff1585e27516c777fb46f'
    $CacheDirectory = Join-Path $Repository 'target\windows-msi\cache'
    New-Item -ItemType Directory -Path $CacheDirectory -Force | Out-Null
    $PythonArchive = Join-Path $CacheDirectory $PythonArchiveName
    if (-not (Test-Path -LiteralPath $PythonArchive -PathType Leaf)) {
        Invoke-WebRequest -UseBasicParsing -Uri $PythonArchiveUrl -OutFile $PythonArchive
    }
    $ActualPythonArchiveHash = (Get-FileHash -LiteralPath $PythonArchive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($ActualPythonArchiveHash -ne $PythonArchiveSha256) {
        Remove-Item -LiteralPath $PythonArchive -Force
        throw "El SHA-256 del runtime Python no coincide ($ActualPythonArchiveHash)."
    }
    & tar.exe -xzf $PythonArchive -C $RuntimeExtract
    if ($LASTEXITCODE -ne 0) { throw "No pude extraer CPython ($LASTEXITCODE)." }
    $ExtractedPython = Join-Path $RuntimeExtract 'python'
    if (-not (Test-Path -LiteralPath (Join-Path $ExtractedPython 'python.exe') -PathType Leaf)) {
        throw "El paquete CPython no contiene python/python.exe: $PythonArchiveName"
    }
    Move-Item -LiteralPath $ExtractedPython -Destination $RuntimeDirectory
    Copy-Item -LiteralPath $Uv.Source -Destination (Join-Path $RuntimeDirectory 'uv.exe')
    New-Item -ItemType Directory -Path $SitePackages -Force | Out-Null

    Push-Location (Join-Path $Repository 'old-version')
    try {
        & $Uv.Source export --quiet --no-progress --locked --no-dev --no-emit-project --no-annotate --format requirements.txt --output-file $Requirements
        if ($LASTEXITCODE -ne 0) { throw "uv export falló con código $LASTEXITCODE." }
        & $Uv.Source build --quiet --no-build-logs --wheel --out-dir $WheelDirectory
        if ($LASTEXITCODE -ne 0) { throw "uv build del helper falló con código $LASTEXITCODE." }
    } finally {
        Pop-Location
    }
    $Wheels = @(Get-ChildItem -LiteralPath $WheelDirectory -Filter '*.whl' -File)
    if ($Wheels.Count -ne 1) { throw "Se esperaba un wheel del helper y encontré $($Wheels.Count)." }

    $PythonExe = Join-Path $RuntimeDirectory 'python.exe'
    & $Uv.Source pip install --python $PythonExe --target $SitePackages --require-hashes --only-binary ':all:' --requirements $Requirements --no-progress
    if ($LASTEXITCODE -ne 0) { throw "uv pip install de dependencias falló con código $LASTEXITCODE." }
    & $Uv.Source pip install --python $PythonExe --target $SitePackages --no-deps $Wheels[0].FullName --no-progress
    if ($LASTEXITCODE -ne 0) { throw "uv pip install del helper falló con código $LASTEXITCODE." }

    & cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build de Studio falló con código $LASTEXITCODE." }
    & cargo build --locked --release --no-default-features --features python-helper-launcher --bin chxchx-python-helper
    if ($LASTEXITCODE -ne 0) { throw "cargo build del launcher Python falló con código $LASTEXITCODE." }
    $Bin = Join-Path $Repository 'target\release'
    $BinDirectory = Join-Path $Payload 'bin'
    New-Item -ItemType Directory -Path $BinDirectory -Force | Out-Null
    $CommandExecutables = @(Get-ChildItem -LiteralPath $Bin -File -Filter 'chxchx*.exe' | Where-Object {
        $_.Name -notin @('chxchx-studio.exe', 'chxchx-installer.exe', 'chxchx-python-helper.exe')
    })
    foreach ($Name in 'chxchx-studio.exe', 'chxchx-tech.exe', 'chxchx.exe') {
        $Source = Join-Path $Bin $Name
        if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Falta $Source." }
    }
    Copy-Item -LiteralPath (Join-Path $Bin 'chxchx-studio.exe') -Destination $BinDirectory
    foreach ($Executable in $CommandExecutables) {
        Copy-Item -LiteralPath $Executable.FullName -Destination $BinDirectory
    }
    Copy-Item -LiteralPath (Join-Path $Bin 'chxchx-python-helper.exe') -Destination (Join-Path $ScriptsDirectory 'chxchx-tech.exe')
    & (Join-Path $Repository 'scripts\verify_windows_binaries.ps1') -BinDirectory $Bin

    $CommitText = $Commit + "`n"
    $MarkerEncoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText((Join-Path $Payload '.chxchx-managed'), $CommitText, $MarkerEncoding)
    [System.IO.File]::WriteAllText((Join-Path $Payload '.chxchx-msi'), "channel=$Channel`nversion=$Version`ncommit=$Commit`n", $MarkerEncoding)

    $Shims = Join-Path $Payload 'shims'
    New-Item -ItemType Directory -Path $Shims -Force | Out-Null
    $ChannelLabel = if ($Channel -eq 'dev') { 'dev' } else { 'stable' }
    $LauncherName = if ($Channel -eq 'dev') { 'chxchx-studio-dev.cmd' } else { 'chxchx-studio.cmd' }
    $LauncherMarks = @("rem CHXCHX_STUDIO_LAUNCHER channel=$ChannelLabel")
    if ($Channel -eq 'dev') { $LauncherMarks += 'rem CHXCHX_STUDIO_DEV_LAUNCHER' }
    $LauncherLines = @('@echo off') + $LauncherMarks + @(
        'set "APP_ROOT=%~dp0.."',
        'set "CHXCHX_TECH_CLI=%APP_ROOT%\.venv\Scripts\chxchx-tech.exe"',
        '"%APP_ROOT%\bin\chxchx-studio.exe" %*'
    )
    [System.IO.File]::WriteAllLines((Join-Path $Shims $LauncherName), $LauncherLines, [System.Text.Encoding]::ASCII)
    foreach ($Executable in $CommandExecutables) {
        $Command = [IO.Path]::GetFileNameWithoutExtension($Executable.Name)
        $Lines = @(
            '@echo off',
            'rem CHXCHX_CLI_SHIM',
            'set "APP_ROOT=%~dp0.."',
            'set "CHXCHX_TECH_CLI=%APP_ROOT%\.venv\Scripts\chxchx-tech.exe"',
            '"%APP_ROOT%\bin\' + $Executable.Name + '" %*'
        )
        [System.IO.File]::WriteAllLines((Join-Path $Shims "$Command.cmd"), $Lines, [System.Text.Encoding]::ASCII)
    }

    $Helper = Join-Path $ScriptsDirectory 'chxchx-tech.exe'
    & $Helper --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'El helper Python incluido no responde a --help.' }
    & (Join-Path $BinDirectory 'chxchx-tech.exe') --version | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'El CLI Rust no responde a --version.' }
    & $PythonExe -c 'import chxchx_tech_lead; from importlib.metadata import version; print(version("chxchx-tech-lead"))' | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'El runtime Python incluido no puede importar el paquete helper.' }

    $Wxs = Join-Path $Work 'ChxChxStudio.wxs'
    & $Python.Source (Join-Path $Repository 'scripts\generate_windows_msi_wxs.py') `
        --payload $Payload --output $Wxs --project-root $Repository --channel $Channel --version $Version --commit $Commit
    if ($LASTEXITCODE -ne 0) { throw 'Falló la generación de la fuente WiX.' }

    $Output = Join-Path $Repository $OutputDirectory
    New-Item -ItemType Directory -Path $Output -Force | Out-Null
    $Msi = Join-Path $Output 'chxchx-studio-windows-x86_64.msi'
    Invoke-WixChecked -Arguments @(
        'build', $Wxs, '-arch', 'x64', '-d', "PayloadDir=$Payload",
        '-d', "ProjectRoot=$Repository", '-dcl', 'high', '-o', $Msi
    ) -LogPath (Join-Path $Work 'wix-build.log') -Action 'la construcción del MSI principal'
    Invoke-WixChecked -Arguments @('msi', 'validate', $Msi, '-sice', 'ICE91') `
        -LogPath (Join-Path $Work 'wix-validate.log') -Action 'la validación ICE del MSI principal'

    if ($BuildSmokeUpdate) {
        $UpdateVersion = $Version
        if ($Channel -eq 'stable') {
            $VersionParts = @($Version.Split('.') | ForEach-Object { [int]$_ })
            if ($VersionParts[2] -ge 65535) { throw 'No puedo crear una versión MSI de actualización de prueba.' }
            $VersionParts[2]++
            $UpdateVersion = $VersionParts -join '.'
        }
        $UpdateCommitCharacter = if ($Commit.StartsWith('a')) { 'b' } else { 'a' }
        $UpdateCommit = $UpdateCommitCharacter * 40
        [System.IO.File]::WriteAllText((Join-Path $Payload '.chxchx-managed'), $UpdateCommit + "`n", $MarkerEncoding)
        [System.IO.File]::WriteAllText(
            (Join-Path $Payload '.chxchx-msi'),
            "channel=$Channel`nversion=$UpdateVersion`ncommit=$UpdateCommit`n",
            $MarkerEncoding
        )
        & $Python.Source (Join-Path $Repository 'scripts\generate_windows_msi_wxs.py') `
            --payload $Payload --output $Wxs --project-root $Repository --channel $Channel `
            --version $UpdateVersion --commit $UpdateCommit
        if ($LASTEXITCODE -ne 0) { throw 'Falló la generación de la fuente WiX de actualización.' }
        $SmokeOutput = Join-Path $Repository 'dist/windows-msi-smoke-update'
        New-Item -ItemType Directory -Path $SmokeOutput -Force | Out-Null
        $SmokeMsi = Join-Path $SmokeOutput 'chxchx-studio-windows-x86_64.msi'
        Invoke-WixChecked -Arguments @(
            'build', $Wxs, '-arch', 'x64', '-d', "PayloadDir=$Payload",
            '-d', "ProjectRoot=$Repository", '-dcl', 'high', '-o', $SmokeMsi
        ) -LogPath (Join-Path $Work 'wix-smoke-build.log') -Action 'la construcción del MSI de actualización'
        Invoke-WixChecked -Arguments @('msi', 'validate', $SmokeMsi, '-sice', 'ICE91') `
            -LogPath (Join-Path $Work 'wix-smoke-validate.log') -Action 'la validación ICE del MSI de actualización'
        Write-Host "MSI de prueba de actualización: $SmokeMsi"
        Write-Host "Versión de prueba: $UpdateVersion; commit de prueba: $UpdateCommit"
    }

    $Success = $true
    Write-Host "MSI generado: $Msi"
    Write-Host "Canal: $Channel; versión: $Version; raíz: $ChannelLabel"
} finally {
    if ($Success -and (Test-Path -LiteralPath $WorkRoot)) {
        $ResolvedWork = [System.IO.Path]::GetFullPath($WorkRoot)
        if (-not $ResolvedWork.StartsWith($TemporaryRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "No limpié una carpeta de trabajo fuera de la raíz temporal autorizada: $ResolvedWork"
        }
        $CleanupError = $null
        for ($Attempt = 1; $Attempt -le 5; $Attempt++) {
            try {
                Remove-Item -LiteralPath $ResolvedWork -Recurse -Force -ErrorAction Stop
                $CleanupError = $null
                break
            } catch {
                $CleanupError = $_
                if ($Attempt -lt 5) { Start-Sleep -Seconds ([Math]::Min(2 * $Attempt, 10)) }
            }
        }
        if ($CleanupError) {
            Write-Warning "MSI generado; no pude limpiar archivos temporales tras cinco intentos: $($CleanupError.Exception.Message)"
        }
    } elseif (Test-Path -LiteralPath $WorkRoot) {
        Write-Warning "Conservé los archivos de trabajo para diagnosticar el error: $WorkRoot"
    }
}
