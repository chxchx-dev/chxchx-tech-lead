param(
    [string]$BinDirectory = "target/release"
)

$ErrorActionPreference = 'Stop'

$metadataJson = & cargo metadata --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) { throw "cargo metadata falló ($LASTEXITCODE)." }
$metadata = $metadataJson | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'chxchx-tech' | Select-Object -First 1).version
if (-not $version) { throw 'No se encontró la versión del paquete chxchx-tech.' }
$numericVersion = ($version -split '[+-]', 2)[0]
$expectedFileVersion = $numericVersion

$kitRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$manifestTool = Get-ChildItem -LiteralPath $kitRoot -Filter mt.exe -Recurse -ErrorAction SilentlyContinue |
    Where-Object FullName -match '\\x64\\mt\.exe$' |
    Sort-Object { [version]($_.Directory.Parent.Name) } -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $manifestTool) { throw "No se encontró mt.exe en $kitRoot." }

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$dumpbin = $null
if (Test-Path -LiteralPath $vswhere) {
    $visualStudio = & $vswhere -latest -products '*' -property installationPath
    if ($visualStudio) {
        $msvcVersions = Join-Path $visualStudio 'VC\Tools\MSVC'
        $msvcVersion = Get-ChildItem -LiteralPath $msvcVersions -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending | Select-Object -First 1
        if ($msvcVersion) {
            $candidate = Join-Path $msvcVersion.FullName 'bin\Hostx64\x64\dumpbin.exe'
            if (Test-Path -LiteralPath $candidate) { $dumpbin = $candidate }
        }
    }
}
if (-not $dumpbin) { throw 'No se encontró dumpbin.exe para verificar las dependencias de runtime.' }

$directory = (Resolve-Path -LiteralPath $BinDirectory).Path
$names = @('chxchx-studio.exe', 'chxchx-tech.exe', 'chxchx.exe')
if (Test-Path -LiteralPath (Join-Path $directory 'chxchx-agent-adapter.exe') -PathType Leaf) {
    $names += 'chxchx-agent-adapter.exe'
}
foreach ($name in $names) {
    $executable = Join-Path $directory $name
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "No se encontró $executable."
    }

    $info = (Get-Item -LiteralPath $executable).VersionInfo
    if ($info.FileVersion -ne $expectedFileVersion -or $info.ProductName -ne 'ChxChx') {
        throw "$name tiene metadatos inesperados: versión '$($info.FileVersion)', producto '$($info.ProductName)'."
    }
    if (-not $info.ProductVersion.StartsWith($version, [StringComparison]::Ordinal)) {
        throw "$name no declara ProductVersion $version (actual: '$($info.ProductVersion)')."
    }

    $manifestPath = Join-Path ([IO.Path]::GetTempPath()) "chxchx-$([guid]::NewGuid().ToString('N')).manifest"
    try {
        & $manifestTool -nologo "-inputresource:$executable;#1" "-out:$manifestPath" | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "mt.exe no pudo extraer el manifiesto de $name ($LASTEXITCODE)." }
        [xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
        $executionLevel = $manifest.SelectSingleNode("//*[local-name()='requestedExecutionLevel']")
        $dpiAwareness = $manifest.SelectSingleNode("//*[local-name()='dpiAwareness']")
        $longPathAware = $manifest.SelectSingleNode("//*[local-name()='longPathAware']")
        if ($executionLevel.level -ne 'asInvoker' -or $dpiAwareness.InnerText -notmatch 'PerMonitorV2' -or $longPathAware.InnerText -ne 'true') {
            throw "$name no contiene ejecución sin elevación, DPI PerMonitorV2 y longPathAware."
        }
    }
    finally {
        Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
    }

    $dependencies = & $dumpbin /dependents $executable | Out-String
    if ($LASTEXITCODE -ne 0) { throw "dumpbin no pudo inspeccionar $name ($LASTEXITCODE)." }
    if ($dependencies -match '(?im)\b(?:VCRUNTIME140(?:_1)?|MSVCP140(?:_[^.]*)?)\.dll\b') {
        throw "$name depende del Visual C++ Redistributable: $($Matches[0])."
    }
    Write-Host "OK: $name version=$($info.FileVersion), manifiesto presente, runtime MSVC estático."
}
