; Questkeep registers itself for questkeep:// links in HKCU at every start (src/protocol.rs);
; the uninstaller removes that key again.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\questkeep"
!macroend
