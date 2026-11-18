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
; Opening audio files: the file type LMP opens, the app's entry for "Open with" and its
; capabilities for "Default apps" in Settings
!define PROG_ID "LMP.AudioFile"
!define APP_KEY "Software\Classes\Applications\${APP_EXE}"
!define CAPABILITIES_KEY "Software\${APP_NAME}\Capabilities"
; SHChangeNotify(SHCNE_ASSOCCHANGED): Explorer re-reads the file associations right away
!define NOTIFY_ASSOCIATIONS "shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)"

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

; The extensions LMP opens — the same as SUPPORTED_EXTENSIONS in src/core/library/scan.rs.
; ${Op} is a macro called for each of them
!macro ForEachAudioExtension Op
  !insertmacro ${Op} ".mp3"
  !insertmacro ${Op} ".flac"
  !insertmacro ${Op} ".wav"
  !insertmacro ${Op} ".ogg"
  !insertmacro ${Op} ".m4a"
  !insertmacro ${Op} ".aac"
!macroend

; LMP goes into the extension's "Open with" list. The default app isn't changed: the user
; picks LMP there or in "Default apps"
!macro RegisterExtension Ext
  WriteRegStr HKLM "Software\Classes\${Ext}\OpenWithProgids" "${PROG_ID}" ""
  WriteRegStr HKLM "${APP_KEY}\SupportedTypes" "${Ext}" ""
  WriteRegStr HKLM "${CAPABILITIES_KEY}\FileAssociations" "${Ext}" "${PROG_ID}"
!macroend

!macro UnregisterExtension Ext
  DeleteRegValue HKLM "Software\Classes\${Ext}\OpenWithProgids" "${PROG_ID}"
  DeleteRegKey /ifempty HKLM "Software\Classes\${Ext}\OpenWithProgids"
!macroend

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

Section "Open audio files with LMP" SecFileTypes
  ; The file type: how LMP opens a file (the app takes paths as arguments, see src/ui/open_with.rs)
  WriteRegStr HKLM "Software\Classes\${PROG_ID}" "" "Audio file"
  WriteRegStr HKLM "Software\Classes\${PROG_ID}\DefaultIcon" "" "$INSTDIR\${APP_EXE},0"
  WriteRegStr HKLM "Software\Classes\${PROG_ID}\shell\open\command" "" '"$INSTDIR\${APP_EXE}" "%1"'

  ; The app's entry: its name in "Open with"
  WriteRegStr HKLM "${APP_KEY}" "FriendlyAppName" "${APP_NAME}"
  WriteRegStr HKLM "${APP_KEY}\DefaultIcon" "" "$INSTDIR\${APP_EXE},0"
  WriteRegStr HKLM "${APP_KEY}\shell\open\command" "" '"$INSTDIR\${APP_EXE}" "%1"'

  ; Capabilities: LMP is listed in Settings → Default apps
  WriteRegStr HKLM "${CAPABILITIES_KEY}" "ApplicationName" "${APP_NAME}"
  WriteRegStr HKLM "${CAPABILITIES_KEY}" "ApplicationDescription" "Audio player"
  WriteRegStr HKLM "Software\RegisteredApplications" "${APP_NAME}" "${CAPABILITIES_KEY}"

  !insertmacro ForEachAudioExtension RegisterExtension
  System::Call "${NOTIFY_ASSOCIATIONS}"
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

  ; Opening audio files — whether or not it was chosen at install
  !insertmacro ForEachAudioExtension UnregisterExtension
  DeleteRegKey HKLM "Software\Classes\${PROG_ID}"
  DeleteRegKey HKLM "${APP_KEY}"
  DeleteRegKey HKLM "Software\${APP_NAME}"
  DeleteRegValue HKLM "Software\RegisteredApplications" "${APP_NAME}"
  System::Call "${NOTIFY_ASSOCIATIONS}"
SectionEnd
