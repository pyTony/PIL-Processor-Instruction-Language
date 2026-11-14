' PIL DOS desktop launcher (Windows).
' Default: the Rust PILdos setup screen, pil.exe --pildos, in its own window.
' It edits pildos.conf (or PILDOS_CONFIG); F10 starts the configured program.
' Extra arguments go to --pildos, e.g. a CONFIG path or --tty.
'   --rust-text ...  pil.exe --dos-text ... instead
'   --python ...     the older Python/Tk terminal (tools\dos_terminal.py ...)
' PILDOS_ENGINE chooses the pil executable.
Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = dir

engine = WshShell.Environment("PROCESS")("PILDOS_ENGINE")
If Len(engine) = 0 Then
    For Each candidate In Array(dir & "\pil.exe", dir & "\target\release\pil.exe", dir & "\target\debug\pil.exe")
        If Len(engine) = 0 And fso.FileExists(candidate) Then engine = candidate
    Next
End If

route = "pildos"
first = 0
If WScript.Arguments.Count > 0 Then
    If WScript.Arguments(0) = "--python" Then route = "python": first = 1
    If WScript.Arguments(0) = "--rust-text" Then route = "text": first = 1
End If

If route <> "python" Then
    If Len(engine) = 0 Then
        MsgBox "No PIL engine in " & dir & vbCrLf & vbCrLf & "Build it there first:" & vbCrLf & "cargo build --release --locked", vbCritical, "PIL DOS startup error"
        WScript.Quit 1
    End If
    If route = "text" Then
        cmd = Chr(34) & engine & Chr(34) & " --dos-text"
        windowStyle = 1
    Else
        cmd = Chr(34) & engine & Chr(34) & " --pildos"
        windowStyle = 0
    End If
    For i = first To WScript.Arguments.Count - 1
        arg = WScript.Arguments(i)
        If arg = "--tty" Then windowStyle = 1
        cmd = cmd & " " & Chr(34) & Replace(arg, Chr(34), Chr(34) & Chr(34)) & Chr(34)
    Next
Else
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

    For i = first To WScript.Arguments.Count - 1
        arg = WScript.Arguments(i)
        If arg = "--headless" Then windowStyle = 1
        cmd = cmd & " " & Chr(34) & Replace(arg, Chr(34), Chr(34) & Chr(34)) & Chr(34)
    Next
End If

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
