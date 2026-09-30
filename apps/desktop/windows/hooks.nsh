; The terminals outlive devpit's window: psmux, from devpit's own bin\, keeps
; running after the app closes. Windows will not replace or remove a file a
; process is running from, so installing over devpit, or removing it, first
; ends that psmux — only the one under this install, never another.
!macro DEVPIT_END_PSMUX
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -Command "Get-Process tmux,psmux,pmux -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like ''$INSTDIR\bin\*'' } | Stop-Process -Force"'
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro DEVPIT_END_PSMUX
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro DEVPIT_END_PSMUX
!macroend
