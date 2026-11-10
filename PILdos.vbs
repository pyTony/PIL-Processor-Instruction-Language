Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = dir

config = WshShell.Environment("PROCESS")("PILDOS_CONFIG")
If fso.FileExists("pildos.conf") Or Len(config) > 0 Then
    cmd = "python tools/dos_terminal.py"
Else
    cmd = "python tools/dos_terminal.py --country 358 --mount DriveD --args ""/K D:\AUTOEXEC.BAT"" images/FD14LITE.img::COMMAND.COM"
End If

For Each arg In WScript.Arguments
    cmd = cmd & " " & Chr(34) & Replace(arg, Chr(34), Chr(34) & Chr(34)) & Chr(34)
Next

On Error Resume Next
result = WshShell.Run(cmd, 0, True)
If Err.Number <> 0 Then
    MsgBox "Could not start PIL DOS in " & dir & vbCrLf & Err.Description & vbCrLf & vbCrLf & "Run this command in that folder to see details:" & vbCrLf & cmd, vbCritical, "PIL DOS startup error"
ElseIf result <> 0 Then
    MsgBox "PIL DOS exited with code " & result & " in " & dir & vbCrLf & vbCrLf & "Run this command in that folder to see details:" & vbCrLf & cmd, vbCritical, "PIL DOS startup error"
End If
On Error GoTo 0
Set WshShell = Nothing
Set fso = Nothing
