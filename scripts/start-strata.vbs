' Strata (Rust) desktop launcher.
' Starts the newest built ka-hgis.exe (release or debug, like run-strata.ps1)
' with the repo root as working directory and without a console window.
Option Explicit

Dim fso, sh, root, exe, cand, f, best
Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("WScript.Shell")

root = fso.GetParentFolderName(fso.GetParentFolderName(WScript.ScriptFullName))

best = ""
For Each cand In Array(root & "\target\release\ka-hgis.exe", root & "\target\debug\ka-hgis.exe")
  If fso.FileExists(cand) Then
    If best = "" Then
      best = cand
    ElseIf fso.GetFile(cand).DateLastModified > fso.GetFile(best).DateLastModified Then
      best = cand
    End If
  End If
Next

If best = "" Then
  MsgBox "Strata exe not found." & vbCrLf & "Build first: cargo build --release -p strata-app --bin ka-hgis", vbExclamation, "Strata"
  WScript.Quit 1
End If

sh.CurrentDirectory = root
sh.Run """" & best & """", 0, False
