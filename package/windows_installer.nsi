; Example NSIS script - Prettier Version
Name SamRewritten
!define APP_NAME "SamRewritten"
!define APP_VERSION "1.0.0"
!define APP_PUBLISHER "Sam Authors"
!define APP_EXE "samrewritten.exe"
!define APP_EXE_CONSOLE "samrewritten-console.exe"
!define APP_EXE_CLI "samrewritten-cli.exe"

; --- Installer Configuration ---
Outfile "SamRewritten-installer.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
RequestExecutionLevel admin ; Request application privileges

; --- User Interface Enhancements ---
; Modern UI Welcome and Finish pages
!include "MUI2.nsh"
; !define MUI_WELCOMEFINISH_BMPS ".\installer_welcome.bmp" ; Optional: path to a custom welcome bitmap (164x314 pixels)
; !define MUI_UNWELCOMEFINISH_BMPS ".\installer_uninstall.bmp" ; Optional: path to a custom uninstall bitmap
; !define MUI_ABORTWARNING ; Show a warning if the user tries to cancel
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Run SamRewritten now"

; Installer pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Language selection (optional, but good for a "prettier" installer)
!insertmacro MUI_LANGUAGE "English"

; --- Installer Sections ---
Section "Install"
  SetOutPath "$INSTDIR\share"
  File /r /x "icon-theme.cache" "..\SamRewritten-windows-x86_64\share\*.*"

  SetOutPath "$INSTDIR\lib"
  File /r "..\SamRewritten-windows-x86_64\lib\*.*"

  SetOutPath $INSTDIR

  ; Add your files here
  File "..\SamRewritten-windows-x86_64\${APP_EXE}"
  File "..\SamRewritten-windows-x86_64\${APP_EXE_CONSOLE}"
  File "..\SamRewritten-windows-x86_64\${APP_EXE_CLI}"
  File "..\assets\README.txt"
  File "..\LICENSE"
  File /a "..\SamRewritten-windows-x86_64\bin\*.*" ; /a includes all files and subdirectories

  ; Create start menu shortcut
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  ; Create uninstaller
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "InstallLocation" "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Build icon cache
  ExecWait '"$INSTDIR\gtk4-update-icon-cache.exe" -f -t "$INSTDIR\share\icons\hicolor"'
  ExecWait '"$INSTDIR\gtk4-update-icon-cache.exe" -f -t "$INSTDIR\share\icons\Adwaita"'
SectionEnd

; --- "Launch now" Checkbox ---
Function .onInstSuccess
  ; Add a checkbox to launch the application
  ; !insertmacro MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
  ; !insertmacro MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_NAME} now"
FunctionEnd

; --- Uninstaller Section ---
Section "Uninstall"
  Delete "$INSTDIR\*.*"
  RMDir /r "$INSTDIR\bin"
  RMDir /r "$INSTDIR\lib"
  RMDir /r "$INSTDIR\share"
  RMDir "$INSTDIR"

  ; Remove start menu shortcut
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  ; Remove the uninstaller's registry key
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
SectionEnd