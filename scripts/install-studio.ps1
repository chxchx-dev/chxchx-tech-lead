# URL histórica del preview: delega en el instalador unificado (canal dev).
param(
    [switch]$DryRun,
    [switch]$Update
)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$Url = 'https://github.com/chxchx-dev/chxchx-tech-lead/releases/download/studio-dev-preview/install.ps1'
$Script = [scriptblock]::Create((Invoke-RestMethod -UseBasicParsing -Uri $Url))
& $Script -Channel dev -DryRun:$DryRun -Update:$Update
