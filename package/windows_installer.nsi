; Example NSIS script
!define APP_NAME "SamRewritten"
!define APP_VERSION "1.0.0"
!define APP_PUBLISHER "Sam Authors"
!define APP_EXE "samrewritten.exe"

Outfile "SamRewritten-installer.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"

; Request application privileges
RequestExecutionLevel admin

Page directory
Page instfiles

Section "Install"
  SetOutPath $INSTDIR

  ; Add your files here
  File "..\SamRewritten-windows-x86_64\${APP_EXE}"
  File "..\SamRewritten-windows-x86_64\README.txt"
  File "..\SamRewritten-windows-x86_64\bin\*.*"

  ; Create start menu shortcut
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\*.*"
  RMDir "$INSTDIR"

  ; Remove start menu shortcut
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
SectionEnd