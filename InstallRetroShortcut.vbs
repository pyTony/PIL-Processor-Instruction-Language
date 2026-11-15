' Install the native retro browser beside the normal PIL DOS shortcut.
Option Explicit
Dim shell, files, root, link
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
If Not files.FileExists(root & "\PILmedia.vbs") Then
    MsgBox "PILmedia.vbs is missing from this folder.", vbCritical, "PIL Retro"
    WScript.Quit 1
End If
On Error Resume Next
Set link = shell.CreateShortcut(shell.SpecialFolders("Desktop") & "\PIL Retro.lnk")
link.TargetPath = root & "\PILmedia.vbs"
link.WorkingDirectory = root
link.IconLocation = root & "\PILdos.ico,0"
link.Description = "Retro software browser and DOS launch menu"
link.Save
If Err.Number <> 0 Then
    MsgBox "Could not install the shortcut: " & Err.Description, vbCritical, "PIL Retro"
    WScript.Quit 1
End If
MsgBox "PIL Retro was added to the Desktop.", vbInformation, "PIL Retro"
