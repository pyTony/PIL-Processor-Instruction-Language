Set WshShell = CreateObject("WScript.Shell")
dir = CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = dir

cmd = "python tools/dos_terminal.py --engine target/release/pil.exe --country 358 --mount DriveD --args ""/K D:\AUTOEXEC.BAT"" images/FD14LITE.img::COMMAND.COM"

WshShell.Run cmd, 0, False
Set WshShell = Nothing