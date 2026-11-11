; LMP installer for Windows (NSIS).
;
; Build (works on macOS too: brew install makensis):
;   cargo build --release --target x86_64-pc-windows-gnu
;   mkdir -p target/installer
;   makensis -DVERSION=0.1.0 installer/windows.nsi
;
; Output: target/installer/LMP Setup 0.1.0.exe

Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif

!define APP_NAME "LMP"
!define APP_EXE "lmp.exe"
!define PUBLISHER "Fedor Ivanov"
; Registry key for "Add or remove programs"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\LMP"

; Paths are relative to the script folder
!define ROOT ".."
!define APP_BINARY "${ROOT}\target\x86_64-pc-windows-gnu\release\${APP_EXE}"
!define APP_ICON "${ROOT}\assets\images\app_icon.ico"

Name "${APP_NAME}"
OutFile "${ROOT}\target\installer\${APP_NAME} Setup ${VERSION}.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
; If the program is already installed — suggest the same folder
InstallDirRegKey HKLM "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} Setup"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_ABORTWARNING
; On the last page — a "Run LMP" checkbox
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"

; Install for all users (into Program Files, with admin rights),
; so the shortcuts also go to the shared Start menu and desktop, not the current user's
Function .onInit
  SetShellVarContext all
FunctionEnd

Function un.onInit
  SetShellVarContext all
FunctionEnd

Section "${APP_NAME}" SecApp
  ; The program itself is required — the checkbox can't be unchecked
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${APP_BINARY}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"

  ; Entry for "Add or remove programs"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Desktop shortcut" SecDesktop
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  DeleteRegKey HKLM "${UNINSTALL_KEY}"
SectionEnd
