; Instalador gráfico de ChxChx Studio: solo empaqueta install.ps1, que descarga y verifica el release.
; makensis -DCHANNEL=stable|dev [-DOUTPUT=ruta.exe] windows-studio-setup.nsi
Unicode true
RequestExecutionLevel user
!ifndef CHANNEL
  !define CHANNEL "stable"
!endif
!if "${CHANNEL}" == "dev"
  Name "ChxChx Studio Dev"
  Caption "Instalar ChxChx Studio Dev"
!else
  Name "ChxChx Studio"
  Caption "Instalar ChxChx Studio"
!endif
; Paths are relative to this script, whatever directory makensis runs from.
!ifndef OUTPUT
  !define OUTPUT "${__FILEDIR__}\chxchx-studio-windows-x86_64-setup.exe"
!endif
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\ChxChxStudio"
SetCompressor /SOLID lzma
ShowInstDetails show

!include "LogicLib.nsh"
!include "MUI2.nsh"

!define MUI_WELCOMEPAGE_TEXT "Este asistente descarga, verifica e instala ChxChx Studio y el comando chxchx-tech para tu usuario. No requiere permisos de administrador."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Spanish"

Function .onInit
  InitPluginsDir
FunctionEnd

Section "Instalar ChxChx Studio"
  SetOutPath "$PLUGINSDIR"
  File "${__FILEDIR__}\install.ps1"
  DetailPrint "Descargando Studio y sus dependencias."
  ExecWait '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install.ps1" -Channel ${CHANNEL}' $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "La instalación terminó con el código $0. Revisa la ventana de PowerShell y vuelve a ejecutar el instalador."
    SetErrorLevel $0
    Abort
  ${EndIf}
  DetailPrint "ChxChx Studio quedó instalado para este usuario."
SectionEnd
