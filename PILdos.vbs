Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = dir

config = WshShell.Environment("PROCESS")("PILDOS_CONFIG")
confPath = config
If Len(confPath) = 0 Then confPath = "pildos.conf"
hasConf = fso.FileExists(confPath)
If hasConf Or Len(config) > 0 Then
    cmd = "python tools/dos_terminal.py"
Else
    cmd = "python tools/dos_terminal.py --country 358 --mount DriveD --args ""/K D:\AUTOEXEC.BAT"" images/FD14LITE.img::COMMAND.COM"
End If

windowStyle = 0
If hasConf Then
    Set ts = fso.OpenTextFile(confPath, 1)
    Do Until ts.AtEndOfStream
        ln = LCase(Trim(ts.ReadLine))
        If Left(ln, 2) = "#@" Then ln = Trim(Mid(ln, 3))
        If Left(ln, 8) = "headless" Then
            eq = InStr(ln, "=")
            If eq > 0 Then
                val = Trim(Mid(ln, eq + 1))
                If val = "true" Or val = "1" Then windowStyle = 1
            End If
        End If
    Loop
    ts.Close
End If

For Each arg In WScript.Arguments
    If arg = "--headless" Then windowStyle = 1
    cmd = cmd & " " & Chr(34) & Replace(arg, Chr(34), Chr(34) & Chr(34)) & Chr(34)
Next

On Error Resume Next
result = WshShell.Run(cmd, windowStyle, True)
If Err.Number <> 0 Then
    MsgBox "Could not start PIL DOS in " & dir & vbCrLf & Err.Description & vbCrLf & vbCrLf & "Run this command in that folder to see details:" & vbCrLf & cmd, vbCritical, "PIL DOS startup error"
ElseIf result <> 0 Then
    MsgBox "PIL DOS exited with code " & result & " in " & dir & vbCrLf & vbCrLf & "Run this command in that folder to see details:" & vbCrLf & cmd, vbCritical, "PIL DOS startup error"
End If
On Error GoTo 0
Set WshShell = Nothing
Set fso = Nothing
