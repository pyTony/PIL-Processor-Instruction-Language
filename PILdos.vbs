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

WshShell.Run cmd, 0, False
Set WshShell = Nothing
Set fso = Nothing
