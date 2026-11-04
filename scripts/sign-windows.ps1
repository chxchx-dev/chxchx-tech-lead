<#
Firma con Authenticode los archivos indicados. CI solo lo ejecuta si existen los secretos
WINDOWS_CERT_PFX_BASE64 y WINDOWS_CERT_PASSWORD; sin certificado los releases salen sin firmar.
Uso: sign-windows.ps1 -Files ruta1.exe,ruta2.exe
Variables: WINDOWS_CERT_PFX_BASE64, WINDOWS_CERT_PASSWORD, WINDOWS_TIMESTAMP_URL (opcional).
No guarda el certificado en el repositorio: se escribe en un archivo temporal que se borra al terminar.
#>
param([Parameter(Mandatory)][string[]]$Files)
$ErrorActionPreference = 'Stop'
if (-not $env:WINDOWS_CERT_PFX_BASE64 -or -not $env:WINDOWS_CERT_PASSWORD) { throw 'Faltan WINDOWS_CERT_PFX_BASE64 o WINDOWS_CERT_PASSWORD.' }
$Timestamp = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
$SignTool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $SignTool) { throw 'No se encontró signtool.exe (Windows SDK).' }
$Pfx = Join-Path ([System.IO.Path]::GetTempPath()) "chxchx-sign-$PID.pfx"
try {
    [IO.File]::WriteAllBytes($Pfx, [Convert]::FromBase64String($env:WINDOWS_CERT_PFX_BASE64))
    foreach ($File in $Files) {
        & $SignTool.FullName sign /fd SHA256 /tr $Timestamp /td SHA256 /f $Pfx /p $env:WINDOWS_CERT_PASSWORD $File
        if ($LASTEXITCODE -ne 0) { throw "signtool falló con $LASTEXITCODE en $File." }
        & $SignTool.FullName verify /pa $File
        if ($LASTEXITCODE -ne 0) { throw "La firma de $File no verifica." }
    }
} finally {
    if (Test-Path -LiteralPath $Pfx) { Remove-Item -LiteralPath $Pfx -Force }
}
