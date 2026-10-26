Unicode True
!include "MUI2.nsh"
Name "PrintMarten"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\PrintMarten"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "PrintMarten"
    SetOutPath "$INSTDIR"
    File /r "${PAYLOAD}\*"
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    CreateDirectory "$SMPROGRAMS\PrintMarten"
    CreateShortcut "$SMPROGRAMS\PrintMarten\PrintMarten.lnk" "$INSTDIR\printmarten.exe"
    CreateShortcut "$SMPROGRAMS\PrintMarten\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "DisplayName" "PrintMarten"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "DisplayIcon" "$INSTDIR\printmarten.exe"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten" "NoRepair" 1
SectionEnd

Section "Uninstall"
    ; Delete only files installed by PrintMarten. Keep unrelated files in this directory.
    Delete "$INSTDIR\printmarten.exe"
    Delete "$INSTDIR\*.dll"
    RMDir /r "$INSTDIR\Documentation"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"
    Delete "$SMPROGRAMS\PrintMarten\PrintMarten.lnk"
    Delete "$SMPROGRAMS\PrintMarten\Uninstall.lnk"
    RMDir "$SMPROGRAMS\PrintMarten"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\PrintMarten"
SectionEnd
