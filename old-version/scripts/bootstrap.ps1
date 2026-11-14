# Instala solo el comando chxchx-tech (canal estable). No configura nada más:
# cuando quieras preparar tus agentes, ejecuta `chxchx-tech install --dry-run` y luego sin --dry-run.
param([switch]$DryRun)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$Url = 'https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.ps1'
$Script = [scriptblock]::Create((Invoke-RestMethod -UseBasicParsing -Uri $Url))
& $Script -CliOnly -DryRun:$DryRun
