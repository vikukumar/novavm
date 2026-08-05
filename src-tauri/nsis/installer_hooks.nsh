; NovaVM Custom NSIS Installer Hooks
; Handles Add to PATH, Desktop Shortcut, and Run as Administrator privileges.

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Configuring NovaVM installation environment..."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Creating Desktop Shortcut..."
  CreateShortcut "$DESKTOP\NovaVM.lnk" "$INSTDIR\novavm.exe" "" "$INSTDIR\novavm.exe" 0

  DetailPrint "Configuring Administrator Privilege Flags..."
  WriteRegStr HKLM "Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers" "$INSTDIR\novavm.exe" "~ RUNASADMIN"
  WriteRegStr HKCU "Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers" "$INSTDIR\novavm.exe" "~ RUNASADMIN"

  DetailPrint "Adding NovaVM to System PATH environment variable..."
  ReadRegStr $0 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$0;$INSTDIR"
  ReadRegStr $0 HKCU "Environment" "Path"
  WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR"
  SendMessage 0xffff 0x001A 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_UNPOSTINSTALL
  DetailPrint "Removing Desktop Shortcut..."
  Delete "$DESKTOP\NovaVM.lnk"

  DetailPrint "Removing Administrator Privilege Flags..."
  DeleteRegValue HKLM "Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers" "$INSTDIR\novavm.exe"
  DeleteRegValue HKCU "Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers" "$INSTDIR\novavm.exe"
!macroend
