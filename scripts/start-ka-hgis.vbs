' Detached GUI launch. explorer/wscript is outside the agent Job Object,
' so the app stays up after the coding agent shell exits.
Option Explicit
Dim sh, fso, root, ps1, log
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
root = fso.GetParentFolderName(fso.GetParentFolderName(WScript.ScriptFullName))
ps1 = root & "\launch.ps1"
log = root & "\build\Release\ka-hgis-launch.log"
If Not fso.FileExists(ps1) Then
  sh.Popup "launch.ps1 없음: " & ps1, 8, "고고학 전용 HGIS", 16
  WScript.Quit 2
End If
sh.CurrentDirectory = root
sh.Run "powershell.exe -NoProfile -ExecutionPolicy Bypass -File """ & ps1 & """", 0, False
