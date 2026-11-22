[CmdletBinding()]
param(
    [ValidateSet('stable', 'dev')]
    [string]$Channel,
    [string]$Version,
    [string]$Commit,
    [Parameter(Mandatory = $true)]
    [string]$Installer,
    [Parameter(Mandatory = $true)]
    [string]$UpdateInstaller
)

$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$' -or $Commit -notmatch '^[0-9a-f]{40}$') {
    throw 'El test MSI requiere versión X.Y.Z y SHA completo.'
}

$UpdateVersion = $Version
if ($Channel -eq 'stable') {
    $VersionParts = @($Version.Split('.') | ForEach-Object { [int]$_ })
    if ($VersionParts[2] -ge 65535) { throw 'No se puede calcular una versión MSI de actualización.' }
    $VersionParts[2]++
    $UpdateVersion = $VersionParts -join '.'
}
$UpdateCommitCharacter = if ($Commit.StartsWith('a')) { 'b' } else { 'a' }
$UpdateCommit = $UpdateCommitCharacter * 40
$CurrentName = if ($Channel -eq 'stable') { "stable-$Version" } else { "dev-$($Commit.Substring(0, 12))" }
$UpdateName = if ($Channel -eq 'stable') { "stable-$UpdateVersion" } else { "dev-$($UpdateCommit.Substring(0, 12))" }
$CurrentRoot = Join-Path $env:LOCALAPPDATA "Programs\ChxChxStudio\$CurrentName"
$UpdateRoot = Join-Path $env:LOCALAPPDATA "Programs\ChxChxStudio\$UpdateName"
$Label = if ($Channel -eq 'stable') { 'ChxChx Studio' } else { 'ChxChx Studio Dev' }
$StartMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\ChxChx\$Label.lnk"
$LogRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
$LogDirectory = Join-Path $LogRoot 'chxchx-msi-logs'
New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null

function Invoke-Msi([string[]]$Arguments, [string]$LogName) {
    $LogPath = Join-Path $LogDirectory $LogName
    $MsiExec = Join-Path $env:SystemRoot 'System32\msiexec.exe'
    if (-not (Test-Path -LiteralPath $MsiExec -PathType Leaf)) { throw "No se encontró Windows Installer: $MsiExec" }
    $ResolvedArguments = @($Arguments | ForEach-Object {
        if ($_ -match '(?i)\.msi$') { (Resolve-Path -LiteralPath $_).Path } else { $_ }
    })
    $StartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $MsiExec
    $StartInfo.WorkingDirectory = (Get-Location).Path
    $StartInfo.UseShellExecute = $false
    $StartInfo.CreateNoWindow = $true
    $StartInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    foreach ($Argument in $ResolvedArguments + @('/l*v', $LogPath)) {
        [void]$StartInfo.ArgumentList.Add($Argument)
    }
    $Process = [System.Diagnostics.Process]::Start($StartInfo)
    try {
        $Process.WaitForExit()
        $Code = $Process.ExitCode
    } finally { $Process.Dispose() }
    if ($Code -notin @(0, 3010, 1641)) {
        if (Test-Path -LiteralPath $LogPath) { Get-Content -LiteralPath $LogPath -Tail 50 }
        throw "msiexec.exe falló con código $Code; registro: $LogPath"
    }
    return $Code
}

function Invoke-Executable([string]$Executable, [string[]]$Arguments, [string]$Description) {
    $StartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $Executable
    $StartInfo.WorkingDirectory = (Get-Location).Path
    $StartInfo.UseShellExecute = $false
    $StartInfo.CreateNoWindow = $true
    $StartInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    foreach ($Argument in $Arguments) { [void]$StartInfo.ArgumentList.Add($Argument) }
    $Process = [System.Diagnostics.Process]::Start($StartInfo)
    try {
        $Process.WaitForExit()
        if ($Process.ExitCode -ne 0) { throw "$Description terminó con código $($Process.ExitCode)." }
    } finally { $Process.Dispose() }
}

function Get-UserPathParts {
    $Key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
    if ($null -eq $Key) { return @() }
    try {
        return @(([string]$Key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)) -split ';' | Where-Object { $_ })
    } finally { $Key.Close() }
}

function Test-UserPathContains([string]$Target) {
    $Expected = [System.IO.Path]::GetFullPath($Target).TrimEnd('\')
    foreach ($Part in Get-UserPathParts) {
        if ([System.IO.Path]::GetFullPath($Part).TrimEnd('\').Equals($Expected, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $true
        }
    }
    return $false
}

function Assert-AppOpens([string]$Root) {
    $Tech = Join-Path $Root 'bin\chxchx-tech.exe'
    $Helper = Join-Path $Root '.venv\Scripts\chxchx-agent-adapter.exe'
    $Python = Join-Path $Root '.venv\python\python.exe'
    $Studio = Join-Path $Root 'bin\chxchx-studio.exe'
    foreach ($Path in @($Tech, $Helper, $Python, $Studio)) {
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "La instalación está incompleta; falta $Path." }
    }
    Invoke-Executable $Tech @('--version') 'El CLI Rust instalado'
    Invoke-Executable $Helper @('contract') 'El servicio de agentes instalado'
    Invoke-Executable $Python @('--version') 'El runtime Python incluido'
    $StartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $Studio
    $StartInfo.WorkingDirectory = (Get-Location).Path
    $StartInfo.UseShellExecute = $false
    $StartInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    [void]$StartInfo.ArgumentList.Add('.')
    $Process = [System.Diagnostics.Process]::Start($StartInfo)
    try {
        Start-Sleep -Seconds 8
        $Process.Refresh()
        if ($Process.HasExited) { throw "Studio se cerró durante la prueba (código $($Process.ExitCode))." }
    } finally {
        if (-not $Process.HasExited) { $Process.Kill($true) }
        $Process.Dispose()
    }
}

if (Test-Path -LiteralPath $CurrentRoot) { throw "El runner ya tiene una instalación en $CurrentRoot." }
if (Test-Path -LiteralPath $UpdateRoot) { throw "El runner ya tiene una instalación en $UpdateRoot." }
foreach ($Path in @($Installer, $UpdateInstaller)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Falta el MSI de prueba: $Path" }
}

try {
    $CurrentCode = Invoke-Msi @('/i', $Installer, '/qn', '/norestart') 'install.log'
    if (-not (Test-Path -LiteralPath (Join-Path $CurrentRoot '.chxchx-msi') -PathType Leaf)) {
        throw 'La instalación silenciosa no dejó el marcador MSI.'
    }
    if ((Get-Content -LiteralPath (Join-Path $CurrentRoot '.chxchx-managed') -Raw).Trim() -ne $Commit) {
        throw 'El marcador de la primera instalación no coincide con su commit.'
    }
    $SharedTools = Join-Path $env:LOCALAPPDATA 'Programs\ChxChxStudio\tools'
    foreach ($Command in @('basic-memory.exe', 'bm.exe', 'serena.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $SharedTools $Command) -PathType Leaf)) {
            throw "El MSI no instaló el extra requerido $Command en $SharedTools."
        }
    }
    foreach ($Command in @('basic-memory.exe', 'bm.exe', 'serena.exe')) {
        Invoke-Executable (Join-Path $SharedTools $Command) @('--help') "El extra $Command instalado por el MSI"
    }
    $RequiredToolsRoot = Join-Path $env:LOCALAPPDATA 'ChxChxStudio\required-tools'
    if (-not (Get-ChildItem -LiteralPath $RequiredToolsRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'bundle-manifest.txt') } | Select-Object -First 1)) {
        throw 'El MSI no conservó el bundle offline que usan los entornos Basic Memory y Serena.'
    }
    if (-not (Test-UserPathContains $SharedTools)) { throw 'El MSI no agregó los comandos de los extras al PATH de usuario.' }
    if (-not (Test-Path -LiteralPath (Join-Path $CurrentRoot '.venv\python\uv.exe') -PathType Leaf)) {
        throw 'El MSI no incluyó uv para instalar los extras sin depender de una instalación global.'
    }
    if (-not (Test-Path -LiteralPath $StartMenuShortcut -PathType Leaf)) { throw 'No se creó el acceso del menú Inicio.' }
    $CurrentShims = Join-Path $CurrentRoot 'shims'
    if ($CurrentShims -notin (Get-UserPathParts)) { throw 'El MSI no agregó sus shims al PATH de usuario.' }
    $RegistrationKey = "Registry::HKEY_CURRENT_USER\Software\ChxChx\Studio\Installers\$Channel\$CurrentName"
    if (-not (Test-Path -LiteralPath $RegistrationKey)) { throw 'El MSI no registró sus componentes por usuario.' }
    Assert-AppOpens $CurrentRoot

    # Simulates stale/broken command shims from an earlier install: the MSI update must force-replace them.
    foreach ($Command in @('basic-memory.exe', 'bm.exe', 'serena.exe')) {
        Set-Content -LiteralPath (Join-Path $SharedTools $Command) -Value 'stale ChxChx shim' -NoNewline -Encoding Ascii
    }

    $UpdateCode = Invoke-Msi @('/i', $UpdateInstaller, '/qn', '/norestart') 'update.log'
    if (Test-Path -LiteralPath $CurrentRoot) { throw 'La actualización mayor no quitó la carpeta MSI anterior.' }
    if (-not (Test-Path -LiteralPath (Join-Path $UpdateRoot '.chxchx-msi') -PathType Leaf)) {
        throw 'La actualización mayor no instaló la carpeta MSI nueva.'
    }
    if ((Join-Path $CurrentRoot 'shims') -in (Get-UserPathParts)) { throw 'La actualización mantuvo el PATH de la instalación MSI anterior.' }
    if ((Get-Content -LiteralPath (Join-Path $UpdateRoot '.chxchx-managed') -Raw).Trim() -ne $UpdateCommit) {
        throw 'El marcador de la actualización no coincide con su commit.'
    }
    if (-not (Test-Path -LiteralPath $StartMenuShortcut -PathType Leaf)) { throw 'El acceso del menú Inicio desapareció durante la actualización.' }
    if ((Join-Path $UpdateRoot 'shims') -notin (Get-UserPathParts)) { throw 'La actualización no movió el PATH a los shims nuevos.' }
    if (-not (Test-UserPathContains $SharedTools)) { throw 'La actualización quitó el PATH persistente de los extras.' }
    foreach ($Command in @('basic-memory.exe', 'bm.exe', 'serena.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $SharedTools $Command) -PathType Leaf)) {
            throw "La actualización quitó el extra requerido $Command."
        }
        Invoke-Executable (Join-Path $SharedTools $Command) @('--help') "El extra $Command actualizado desde el bundle"
    }
    Assert-AppOpens $UpdateRoot

    $UninstallCode = Invoke-Msi @('/x', $UpdateInstaller, '/qn', '/norestart') 'uninstall.log'
    if (Test-Path -LiteralPath $UpdateRoot) { throw 'La desinstalación silenciosa dejó archivos de la aplicación.' }
    if (Test-Path -LiteralPath $StartMenuShortcut) { throw 'La desinstalación silenciosa dejó el acceso del menú Inicio.' }
    if ((Join-Path $UpdateRoot 'shims') -in (Get-UserPathParts)) { throw 'La desinstalación silenciosa dejó el PATH de ChxChx.' }
    if (Test-Path -LiteralPath $RegistrationKey) { throw 'La desinstalación silenciosa dejó datos de registro del MSI.' }
    if (-not (Test-UserPathContains $SharedTools)) { throw 'La desinstalación quitó el PATH de los extras administrados por uv.' }
    foreach ($Command in @('basic-memory.exe', 'bm.exe', 'serena.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $SharedTools $Command) -PathType Leaf)) {
            throw "La desinstalación quitó el extra administrado por uv: $Command."
        }
    }
    if (-not (Test-Path -LiteralPath $RequiredToolsRoot -PathType Container)) {
        throw 'La desinstalación quitó el bundle que todavía necesita el entorno de herramientas.'
    }
    Write-Host "MSI por usuario: instalar=$CurrentCode, actualizar=$UpdateCode, desinstalar=$UninstallCode."
} catch {
    $Failure = $_
    if (Test-Path -LiteralPath $UpdateRoot) {
        try { Invoke-Msi @('/x', $UpdateInstaller, '/qn', '/norestart') 'cleanup-update.log' }
        catch { Write-Warning "No pude desinstalar el MSI de actualización durante la limpieza: $_" }
    }
    if (Test-Path -LiteralPath $CurrentRoot) {
        try { Invoke-Msi @('/x', $Installer, '/qn', '/norestart') 'cleanup-install.log' }
        catch { Write-Warning "No pude desinstalar el MSI inicial durante la limpieza: $_" }
    }
    throw $Failure
}
