<#
.SYNOPSIS
Instala o actualiza ChxChx Studio y el comando `chxchx-tech` en Windows 10/11.

.DESCRIPTION
No necesita permisos de administrador ni clonar el repositorio. Descarga solo los
paquetes de Windows, comprueba su SHA-256 contra el manifiesto del release e instala
en tu perfil (%LOCALAPPDATA%\Programs\ChxChxStudio).

  & ([scriptblock]::Create((irm https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.ps1)))
  & ([scriptblock]::Create((irm .../install.ps1))) -DryRun
  & ([scriptblock]::Create((irm .../install.ps1))) -Channel dev -Update

.PARAMETER Channel
stable (por defecto): la última versión estable. dev: el preview de la rama dev.

.PARAMETER Version
Instala esa versión estable (X.Y.Z). No se combina con -Channel dev.

.PARAMETER Update
Exige una instalación previa y la actualiza.

.PARAMETER CliOnly
Instala solo el comando chxchx-tech, sin la aplicación de escritorio.

.PARAMETER Uninstall
Quita lo que instaló ChxChx. No toca tus datos ni tus proyectos.

.PARAMETER DryRun
Descarga y verifica (o lista lo que quitaría) sin cambiar nada.
#>
[CmdletBinding()]
param(
    [ValidateSet('stable', 'dev')]
    [string]$Channel = 'stable',
    [string]$Version = '',
    [switch]$Update,
    [switch]$CliOnly,
    [switch]$Uninstall,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5.1 puede no negociar TLS 1.2 por defecto.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Owner = 'chxchx-dev'
$Repository = 'chxchx-tech-lead'
$DevTag = 'studio-dev-preview'
$Releases = "https://github.com/$Owner/$Repository/releases"
$KeepVersions = 2

function Write-Note([string]$Text) { Write-Host $Text }
function Write-Warn([string]$Text) { Write-Warning $Text }

if ($Version -ne '') {
    if ($Channel -ne 'stable') { throw '-Version solo existe en el canal stable.' }
    if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$') { throw 'La versión debe tener la forma X.Y.Z (por ejemplo 0.5.0).' }
}
if ($Uninstall -and ($Update -or $CliOnly -or $Version -ne '')) {
    throw '-Uninstall no se combina con -Update, -CliOnly ni -Version.'
}

$AppParent = Join-Path $env:LOCALAPPDATA 'Programs\ChxChxStudio'
$CliParent = Join-Path $AppParent 'cli'
$ShimDir = Join-Path $AppParent 'shims'
$InstallerDir = Join-Path $AppParent 'installer'
$StartMenuDir = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$ArpKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ChxChxStudio'
$LegacyLauncher = Join-Path $AppParent 'chxchx-studio-dev.cmd'

function Test-ManagedFile([string]$Path, [string[]]$Marks) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    $Item = Get-Item -LiteralPath $Path -Force
    if ($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { return $false }
    $Text = Get-Content -LiteralPath $Path -Raw
    foreach ($Mark in $Marks) { if ($Text -match [regex]::Escape($Mark)) { return $true } }
    return $false
}

function Test-ManagedDirectory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) { return $false }
    $Item = Get-Item -LiteralPath $Path -Force
    if ($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { return $false }
    return (Test-Path -LiteralPath (Join-Path $Path '.chxchx-managed') -PathType Leaf)
}

# ---- PATH del usuario: se edita el registro (conserva %VARIABLES%) y se avisa al sistema ----------------------
function Send-EnvironmentChange {
    if (-not ('ChxChx.Native' -as [type])) {
        Add-Type -Namespace ChxChx -Name Native -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Auto)]
public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
'@
    }
    $Result = [UIntPtr]::Zero
    [void][ChxChx.Native]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$Result)
}

function Get-UserPathKey { return [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true) }

function Add-UserPath([string]$Directory) {
    $Key = Get-UserPathKey
    try {
        $Current = [string]$Key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $Parts = @($Current -split ';' | Where-Object { $_ -ne '' })
        if ($Parts -notcontains $Directory) {
            $Key.SetValue('Path', ((@($Parts) + $Directory) -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
            Send-EnvironmentChange
        }
    } finally { $Key.Close() }
    if (($env:Path -split ';') -notcontains $Directory) { $env:Path = "$Directory;$env:Path" }
}

function Remove-UserPath([string]$Directory) {
    $Key = Get-UserPathKey
    try {
        $Current = [string]$Key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $Parts = @($Current -split ';' | Where-Object { $_ -ne '' })
        if ($Parts -contains $Directory) {
            $Key.SetValue('Path', ((@($Parts | Where-Object { $_ -ne $Directory })) -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
            Send-EnvironmentChange
        }
    } finally { $Key.Close() }
}

# ---- Desinstalación --------------------------------------------------------------------------------------------------------
function Invoke-Uninstall {
    $Removed = 0
    function Remove-Managed([string]$Path) {
        if ($DryRun) { Write-Note "Se quitaría: $Path" } else { Remove-Item -LiteralPath $Path -Recurse -Force; Write-Note "Quitado: $Path" }
        $script:Removed++
    }
    $script:Removed = 0
    foreach ($Parent in @($AppParent, $CliParent)) {
        if (-not (Test-Path -LiteralPath $Parent -PathType Container)) { continue }
        foreach ($Directory in @(Get-ChildItem -LiteralPath $Parent -Directory -Force)) {
            if (Test-ManagedDirectory $Directory.FullName) { Remove-Managed $Directory.FullName }
        }
    }
    foreach ($Name in @('chxchx-studio.cmd', 'chxchx-studio-dev.cmd')) {
        $Path = Join-Path $ShimDir $Name
        if (Test-ManagedFile $Path @('CHXCHX_STUDIO_LAUNCHER', 'CHXCHX_STUDIO_DEV_LAUNCHER')) { Remove-Managed $Path }
    }
    if (Test-ManagedFile $LegacyLauncher @('CHXCHX_STUDIO_DEV_LAUNCHER')) { Remove-Managed $LegacyLauncher }
    $Shim = Join-Path $ShimDir 'chxchx-tech.cmd'
    if (Test-ManagedFile $Shim @('CHXCHX_CLI_SHIM')) { Remove-Managed $Shim }
    foreach ($Name in @('ChxChx Studio.lnk', 'ChxChx Studio Dev.lnk')) {
        $Link = Join-Path $StartMenuDir $Name
        if (Test-Path -LiteralPath $Link -PathType Leaf) {
            $Target = (New-Object -ComObject WScript.Shell).CreateShortcut($Link).TargetPath
            if ($Target -like "$ShimDir*" -or $Target -eq $LegacyLauncher) { Remove-Managed $Link }
        }
    }
    if (-not $DryRun) {
        Remove-UserPath $ShimDir
        if (Test-Path -LiteralPath $ArpKey) { Remove-Item -LiteralPath $ArpKey -Recurse -Force }
        if ((Test-Path -LiteralPath $ShimDir) -and -not @(Get-ChildItem -LiteralPath $ShimDir -Force)) { Remove-Item -LiteralPath $ShimDir -Force }
        if ((Test-Path -LiteralPath $CliParent) -and -not @(Get-ChildItem -LiteralPath $CliParent -Force)) { Remove-Item -LiteralPath $CliParent -Force }
        # El instalador que se está ejecutando puede vivir aquí: se borra al terminar, no mientras corre.
        if (Test-Path -LiteralPath $InstallerDir) {
            Start-Process -FilePath 'cmd.exe' -WindowStyle Hidden -ArgumentList '/c', "ping -n 3 127.0.0.1 >nul & rmdir /s /q `"$InstallerDir`" & rmdir `"$AppParent`" 2>nul"
        } elseif ((Test-Path -LiteralPath $AppParent) -and -not @(Get-ChildItem -LiteralPath $AppParent -Force)) {
            Remove-Item -LiteralPath $AppParent -Force
        }
    }
    if ($script:Removed -eq 0) { Write-Note 'No había nada instalado por ChxChx.' }
    elseif ($DryRun) { Write-Note 'DRY RUN: no se quitó nada.' }
    Write-Note 'No se tocaron tus datos: %USERPROFILE%\.chxchx-tech-lead (confianza, copias, registros) ni la carpeta .ai de tus proyectos.'
    Write-Note 'uv se administra aparte. Para borrar tus datos de ChxChx: Remove-Item -Recurse -Force "$env:USERPROFILE\.chxchx-tech-lead"'
}

if ($Uninstall) { Invoke-Uninstall; return }

# ---- Origen de las descargas -------------------------------------------------------------------------------------------------------
if ([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64 -and -not $CliOnly) {
    throw 'El paquete de escritorio requiere Windows x86_64. Usa -CliOnly para instalar solo el comando chxchx-tech.'
}
$TestBase = $env:CHXCHX_TEST_BASE
if ($TestBase) {
    # Gancho para pruebas: solo un servidor local, y nunca para usuarios normales.
    if ($TestBase -notmatch '^http://(127\.0\.0\.1|localhost):[0-9]+(/[A-Za-z0-9._-]*)*$') {
        throw 'CHXCHX_TEST_BASE solo admite http://127.0.0.1:PUERTO o http://localhost:PUERTO.'
    }
    $Base = $TestBase.TrimEnd('/')
    Write-Warn "Usando una base de pruebas ($Base); no es el origen oficial."
} elseif ($Channel -eq 'dev') {
    $Base = "$Releases/download/$DevTag"
} elseif ($Version -ne '') {
    $Base = "$Releases/download/v$Version"
} else {
    $Base = "$Releases/latest/download"
}

$Platform = 'windows-x86_64'
$ExpectedPackage = 'chxchx-studio-windows-x86_64.zip'
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "chxchx-studio-$PID"
$Stage = $null
New-Item -ItemType Directory -Path $TempRoot | Out-Null
try {
    $CacheBuster = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $ManifestPath = Join-Path $TempRoot 'studio-manifest.txt'
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$Base/studio-manifest.txt?t=$CacheBuster" -OutFile $ManifestPath
    } catch {
        # El preview anterior solo publicaba studio-preview.txt.
        if ($Channel -ne 'dev') { throw "No pude descargar el manifiesto de $Base. ¿Hay conexión, y existe esa versión?" }
        Invoke-WebRequest -UseBasicParsing -Uri "$Base/studio-preview.txt?t=$CacheBuster" -OutFile $ManifestPath
    }
    $Manifest = @{}
    foreach ($Line in Get-Content -LiteralPath $ManifestPath) {
        if ($Line -match '^([a-z0-9_.-]+)=(.*)$') {
            if ($Manifest.ContainsKey($Matches[1])) { throw "Clave repetida en el manifiesto: $($Matches[1])" }
            $Manifest[$Matches[1]] = $Matches[2]
        }
    }
    if ($Manifest['schema_version'] -ne '1') { throw 'Versión de manifiesto no compatible.' }
    $Commit = $Manifest['commit']
    $ManifestVersion = $Manifest['version']
    if ($Commit -notmatch '^[0-9a-f]{40}$') { throw 'El manifiesto contiene un commit inválido.' }
    if ($ManifestVersion -notmatch '^[0-9A-Za-z.+-]+$') { throw 'El manifiesto contiene una versión inválida.' }
    $WheelName = $Manifest['asset.cli-wheel']
    if ($WheelName -notmatch '^chxchx_tech_lead-[A-Za-z0-9.+!_-]+-py3-none-any\.whl$') { throw 'El manifiesto apunta a un wheel inesperado.' }
    $Keys = @('sha256.cli-wheel')
    if (-not $CliOnly) {
        if ($Manifest["asset.$Platform"] -ne $ExpectedPackage) { throw 'El manifiesto apunta a un paquete inesperado.' }
        $Keys += "sha256.$Platform"
    }
    foreach ($Key in $Keys) {
        if ($Manifest[$Key] -notmatch '^[0-9a-f]{64}$') { throw "El manifiesto contiene un checksum inválido: $Key" }
    }
    if (-not $TestBase -and $Manifest.ContainsKey('channel') -and $Manifest['channel'] -ne $Channel) {
        throw "El manifiesto es del canal $($Manifest['channel']), no de $Channel."
    }
    if ($Channel -eq 'stable') {
        if ($ManifestVersion -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$') { throw "La versión estable $ManifestVersion no tiene la forma X.Y.Z." }
        if ($Version -ne '' -and $Version -ne $ManifestVersion) { throw "Pediste $Version pero el release es $ManifestVersion." }
    }

    $ShortCommit = $Commit.Substring(0, 12)
    if ($Channel -eq 'dev') {
        $Id = "dev-$ShortCommit"; $AppLabel = 'ChxChx Studio Dev'; $LauncherName = 'chxchx-studio-dev.cmd'
    } else {
        $Id = "stable-$ManifestVersion"; $AppLabel = 'ChxChx Studio'; $LauncherName = 'chxchx-studio.cmd'
    }
    if ($CliOnly) { $AppRoot = Join-Path $CliParent $Id } else { $AppRoot = Join-Path $AppParent $Id }
    $Launcher = Join-Path $ShimDir $LauncherName
    $Shim = Join-Path $ShimDir 'chxchx-tech.cmd'
    $StartMenu = Join-Path $StartMenuDir "$AppLabel.lnk"
    $CliBinary = Join-Path $AppRoot '.venv\Scripts\chxchx-tech.exe'
    $Python = Join-Path $AppRoot '.venv\Scripts\python.exe'
    $AppBinary = Join-Path $AppRoot 'bin\chxchx-studio.exe'

    # ---- Comprobaciones previas: no se pisa nada ajeno --------------------------------------------------------------------
    $LauncherMarks = @('CHXCHX_STUDIO_LAUNCHER', 'CHXCHX_STUDIO_DEV_LAUNCHER')
    if (-not $CliOnly -and (Test-Path -LiteralPath $Launcher) -and -not (Test-ManagedFile $Launcher $LauncherMarks)) {
        throw "Ya existe $Launcher y no está administrado por ChxChx. No lo sobrescribí."
    }
    if ($Update) {
        $Existing = if ($CliOnly) { Test-ManagedFile $Shim @('CHXCHX_CLI_SHIM') } else { (Test-ManagedFile $Launcher $LauncherMarks) -or (Test-ManagedFile $LegacyLauncher @('CHXCHX_STUDIO_DEV_LAUNCHER')) }
        if (-not $Existing) { throw '-Update requiere una instalación previa administrada por ChxChx.' }
    }
    if (-not $CliOnly -and (Test-Path -LiteralPath $StartMenu)) {
        $ExistingShortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($StartMenu)
        if ($ExistingShortcut.TargetPath -ne $Launcher -and $ExistingShortcut.TargetPath -ne $LegacyLauncher) {
            throw "Ya existe un acceso no administrado: $StartMenu"
        }
    }
    $NeedsCliInstall = $true
    if (Test-Path -LiteralPath $AppRoot) {
        $AppItem = Get-Item -LiteralPath $AppRoot -Force
        if ($AppItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
            throw "La ruta $AppRoot es un enlace o punto de reanálisis. No la modifiqué."
        }
        $Marker = Join-Path $AppRoot '.chxchx-managed'
        if (-not (Test-Path -LiteralPath $Marker -PathType Leaf)) {
            throw "La ruta $AppRoot ya existe y no está administrada por ChxChx. No la modifiqué."
        }
        $ExistingCommit = (Get-Content -LiteralPath $Marker -Raw).Trim()
        if ($ExistingCommit -and $ExistingCommit -ne $Commit) {
            throw "El marcador de $AppRoot corresponde a otra versión. No la modifiqué."
        }
        if (-not $CliOnly -and -not (Test-Path -LiteralPath $AppBinary -PathType Leaf)) {
            throw "La instalación administrada está incompleta: falta $AppBinary."
        }
        if ((Test-Path -LiteralPath $CliBinary -PathType Leaf) -and (Test-Path -LiteralPath $Python -PathType Leaf)) { $NeedsCliInstall = $false }
    }

    # ---- Descarga y verificación ----------------------------------------------------------------------------------------------------
    if ($Channel -eq 'dev') { Write-Note "Preview $ManifestVersion ($ShortCommit) · $Platform" } else { Write-Note "ChxChx $ManifestVersion (canal estable) · $Platform" }
    $WheelPath = Join-Path $TempRoot $WheelName
    Invoke-WebRequest -UseBasicParsing -Uri "$Base/${WheelName}?t=$CacheBuster" -OutFile $WheelPath
    if ((Get-FileHash -LiteralPath $WheelPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Manifest['sha256.cli-wheel']) {
        throw 'El SHA-256 del CLI no coincide; no se instaló nada.'
    }
    if (-not $CliOnly) {
        $PackagePath = Join-Path $TempRoot $ExpectedPackage
        Invoke-WebRequest -UseBasicParsing -Uri "$Base/${ExpectedPackage}?t=$CacheBuster" -OutFile $PackagePath
        if ((Get-FileHash -LiteralPath $PackagePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Manifest["sha256.$Platform"]) {
            throw 'El SHA-256 de Studio no coincide; no se instaló nada.'
        }
    }
    Write-Note "Instalación: $AppRoot"
    if ($DryRun) {
        Write-Note 'DRY RUN: las descargas y checksums son válidos; no se instaló nada ni se escribieron archivos de instalación.'
        return
    }
    if (-not $NeedsCliInstall) {
        $CliReady = $false
        try { & $CliBinary --help | Out-Null; $CliReady = ($LASTEXITCODE -eq 0) } catch { $CliReady = $false }
        if (-not $CliReady) { $NeedsCliInstall = $true }
    }

    # ---- Instalación del paquete ------------------------------------------------------------------------------------------------------
    function Write-Marker([string]$Directory) {
        $Tmp = Join-Path $Directory ".chxchx-managed.tmp-$PID"
        Set-Content -LiteralPath $Tmp -Value $Commit -NoNewline
        Move-Item -LiteralPath $Tmp -Destination (Join-Path $Directory '.chxchx-managed') -Force
        Set-Content -LiteralPath (Join-Path $Directory '.chxchx-install') -Value @("channel=$Channel", "version=$ManifestVersion", "commit=$Commit")
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $AppRoot) -Force | Out-Null
    if ($CliOnly) {
        New-Item -ItemType Directory -Path $AppRoot -Force | Out-Null
        Write-Marker $AppRoot
    } elseif (Test-Path -LiteralPath $AppRoot) {
        Write-Marker $AppRoot
    } else {
        $Stage = Join-Path $AppParent ".chxchx-stage-$ShortCommit-$PID"
        if (Test-Path -LiteralPath $Stage) { Remove-Item -LiteralPath $Stage -Recurse -Force }
        New-Item -ItemType Directory -Path $Stage | Out-Null
        Expand-Archive -LiteralPath $PackagePath -DestinationPath $Stage
        $ExpectedBinary = Join-Path $Stage 'bin\chxchx-studio.exe'
        if (-not (Test-Path -LiteralPath $ExpectedBinary -PathType Leaf)) {
            $PackageRoots = @(Get-ChildItem -LiteralPath $Stage -Directory | Where-Object {
                Test-Path -LiteralPath (Join-Path $_.FullName 'bin\chxchx-studio.exe') -PathType Leaf
            })
            if ($PackageRoots.Count -ne 1) { throw 'El ZIP no contiene una raíz reconocible con bin\chxchx-studio.exe.' }
            foreach ($Child in Get-ChildItem -LiteralPath $PackageRoots[0].FullName -Force) {
                Move-Item -LiteralPath $Child.FullName -Destination $Stage
            }
            Remove-Item -LiteralPath $PackageRoots[0].FullName -Force
        }
        if (-not (Test-Path -LiteralPath $ExpectedBinary -PathType Leaf)) { throw 'El ZIP no contiene bin\chxchx-studio.exe.' }
        Write-Marker $Stage
        Move-Item -LiteralPath $Stage -Destination $AppRoot
        $Stage = $null
    }

    # ---- Comando chxchx-tech (entorno aislado con uv) ------------------------------------------------------------------------------------
    if ($NeedsCliInstall) {
        # Windows PowerShell 5.1 convierte en error cualquier salida por stderr de un programa nativo (uv la usa para el progreso).
        $ErrorActionPreference = 'Continue'
        $Uv = Get-Command uv -ErrorAction SilentlyContinue
        if ($null -eq $Uv) {
            $UvPath = Join-Path $HOME '.local\bin\uv.exe'
            if (-not (Test-Path -LiteralPath $UvPath)) {
                Invoke-RestMethod https://astral.sh/uv/install.ps1 | Invoke-Expression
            }
            if (-not (Test-Path -LiteralPath $UvPath)) { throw 'No se encontró uv después de instalarlo.' }
        } else {
            $UvPath = $Uv.Source
        }
        $PythonEnvironment = Join-Path $AppRoot '.venv'
        if (-not (Test-Path -LiteralPath $Python -PathType Leaf) -or -not (Test-Path -LiteralPath $CliBinary -PathType Leaf)) {
            & $UvPath venv --clear --python 3.11 $PythonEnvironment
            if ($LASTEXITCODE -ne 0) { throw "uv venv falló con código $LASTEXITCODE." }
        }
        & $UvPath pip install --python $Python --reinstall-package chxchx-tech-lead $WheelPath
        if ($LASTEXITCODE -ne 0) { throw "uv pip install falló con código $LASTEXITCODE." }
        if (-not (Test-Path -LiteralPath $CliBinary -PathType Leaf)) { throw 'El wheel no dejó el comando chxchx-tech esperado.' }
        & $CliBinary --help | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'El bridge CLI instalado no puede iniciarse.' }
        $ErrorActionPreference = 'Stop'
    }

    # ---- Launcher, comando en el PATH y acceso del menú Inicio --------------------------------------------------------------------------
    New-Item -ItemType Directory -Path $ShimDir -Force | Out-Null
    function Write-Cmd([string]$Path, [string[]]$Lines) {
        $Tmp = "$Path.tmp-$PID"
        $Lines | Set-Content -LiteralPath $Tmp -Encoding Ascii
        Move-Item -LiteralPath $Tmp -Destination $Path -Force
    }
    if (-not $CliOnly) {
        $Marks = @("rem CHXCHX_STUDIO_LAUNCHER channel=$Channel")
        if ($Channel -eq 'dev') { $Marks += 'rem CHXCHX_STUDIO_DEV_LAUNCHER' }
        Write-Cmd $Launcher (@('@echo off') + $Marks + @(
            "set `"APP_ROOT=$AppRoot`"",
            'set "CHXCHX_TECH_CLI=%APP_ROOT%\.venv\Scripts\chxchx-tech.exe"',
            '"%APP_ROOT%\bin\chxchx-studio.exe" %*'))
    }
    $ShimNote = ''
    if ((Test-Path -LiteralPath $Shim) -and -not (Test-ManagedFile $Shim @('CHXCHX_CLI_SHIM'))) {
        $ShimNote = "Ya existe $Shim y no es de ChxChx; no lo reemplacé. El comando de esta instalación está en $CliBinary"
    } else {
        Write-Cmd $Shim @('@echo off', 'rem CHXCHX_CLI_SHIM', "`"$CliBinary`" %*")
    }
    Add-UserPath $ShimDir

    if (-not $CliOnly) {
        New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
        $Shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($StartMenu)
        $Shortcut.TargetPath = $Launcher
        $Shortcut.WorkingDirectory = $AppRoot
        $Shortcut.IconLocation = "$AppBinary,0"
        $Shortcut.Description = "$AppLabel $ManifestVersion"
        $Shortcut.WindowStyle = 7
        $Shortcut.Save()
        # El acceso antiguo (preview anterior) apuntaba al launcher de la carpeta raíz: se retira si era de ChxChx.
        if (Test-ManagedFile $LegacyLauncher @('CHXCHX_STUDIO_DEV_LAUNCHER')) { Remove-Item -LiteralPath $LegacyLauncher -Force }
    }

    # ---- Una copia del instalador y la entrada en "Aplicaciones instaladas" para poder desinstalar -----------------------------------------
    $Registered = $false
    if ($Manifest.ContainsKey('asset.install-ps1') -and -not $CliOnly) {
        New-Item -ItemType Directory -Path $InstallerDir -Force | Out-Null
        $InstallerCopy = Join-Path $InstallerDir 'install.ps1'
        $Downloaded = Join-Path $TempRoot 'install.ps1'
        Invoke-WebRequest -UseBasicParsing -Uri "$Base/install.ps1?t=$CacheBuster" -OutFile $Downloaded
        if ((Get-FileHash -LiteralPath $Downloaded -Algorithm SHA256).Hash.ToLowerInvariant() -eq $Manifest['sha256.install-ps1']) {
            Copy-Item -LiteralPath $Downloaded -Destination $InstallerCopy -Force
            Set-Content -LiteralPath (Join-Path $InstallerDir '.chxchx-managed') -Value $Commit -NoNewline
            $Registered = $true
        } else {
            Write-Warn 'La copia del instalador no coincide con el manifiesto; no registré la desinstalación en Windows.'
        }
    }
    if ($Registered) {
        New-Item -Path $ArpKey -Force | Out-Null
        $Arp = @{
            DisplayName = $AppLabel; DisplayVersion = $ManifestVersion; Publisher = 'ChxChx Tech Lead'
            InstallLocation = $AppRoot; DisplayIcon = $AppBinary; NoModify = 1; NoRepair = 1
            UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $InstallerDir 'install.ps1')`" -Uninstall"
        }
        foreach ($Name in $Arp.Keys) { Set-ItemProperty -Path $ArpKey -Name $Name -Value $Arp[$Name] }
    }

    # ---- Limpieza: se conservan la versión nueva y la anterior (para volver atrás) ----------------------------------------------------------
    (Get-Item -LiteralPath $AppRoot).LastWriteTime = Get-Date
    $PruneParent = Split-Path -Parent $AppRoot
    $Old = @(Get-ChildItem -LiteralPath $PruneParent -Directory -Filter "$Channel-*" -Force | Sort-Object LastWriteTime -Descending | Select-Object -Skip $KeepVersions)
    foreach ($Directory in $Old) {
        if (Test-ManagedDirectory $Directory.FullName) { Remove-Item -LiteralPath $Directory.FullName -Recurse -Force }
    }

    Write-Note ''
    Write-Note "ChxChx $ManifestVersion quedó instalado."
    if (-not $CliOnly) { Write-Note "Abre Studio desde el menú Inicio ($AppLabel) o con: $($LauncherName -replace '\.cmd$', '') C:\ruta\al\proyecto" }
    Write-Note 'Comando: chxchx-tech doctor   (abre una terminal nueva si aún no lo encuentra)'
    if ($ShimNote) { Write-Warn $ShimNote }
    if (-not $CliOnly) { Write-Note 'El preview no está firmado: Windows puede mostrar SmartScreen la primera vez ("Más información" > "Ejecutar de todas formas").' }
    Write-Note 'Para desinstalar: "Aplicaciones instaladas" de Windows, o install.ps1 -Uninstall'
} finally {
    if ($null -ne $Stage -and (Test-Path -LiteralPath $Stage)) { Remove-Item -LiteralPath $Stage -Recurse -Force }
    if (Test-Path -LiteralPath $TempRoot) { Remove-Item -LiteralPath $TempRoot -Recurse -Force }
}
