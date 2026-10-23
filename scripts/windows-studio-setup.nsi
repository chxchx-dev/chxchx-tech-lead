Unicode true
RequestExecutionLevel user
Name "ChxChx Studio Dev"
Caption "Instalar ChxChx Studio Dev"
OutFile "scripts/chxchx-studio-windows-x86_64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\ChxChxStudio"
SetCompressor /SOLID lzma
ShowInstDetails show

!include "LogicLib.nsh"
!include "MUI2.nsh"

!define MUI_WELCOMEPAGE_TEXT "Este asistente descarga e instala ChxChx Studio y el bridge CLI para tu usuario. No requiere permisos de administrador."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Spanish"

Function .onInit
  InitPluginsDir
FunctionEnd

Section "Instalar ChxChx Studio"
  SetOutPath "$PLUGINSDIR"
  File "scripts/install-studio.ps1"
  DetailPrint "Descargando Studio y sus dependencias."
  ExecWait '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-studio.ps1"' $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "La instalación terminó con el código $0. Revisa la ventana de PowerShell y vuelve a ejecutar el instalador."
    SetErrorLevel $0
    Abort
  ${EndIf}
  DetailPrint "ChxChx Studio quedó instalado para este usuario."
SectionEnd
