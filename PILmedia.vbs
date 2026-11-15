' Native catalog launcher. Hide the engine console, not its interactive window.
Option Explicit
Dim shell, files, root, engine, catalog, command, result, checkOnly, runtimeFolder, runtimeCopy, builtAt, stale
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = root
engine = root & "\target\release\pil.exe"
catalog = root & "\images\retro\catalog.tsv"
checkOnly = False
If WScript.Arguments.Count > 0 Then checkOnly = (WScript.Arguments(0) = "--check")
If Not files.FileExists(engine) Or Not files.FileExists(catalog) Then
    If Not checkOnly Then MsgBox "The runtime or catalog is missing. Build with cargo build --release --locked and check images/retro/catalog.tsv.", vbCritical, "Retro browser"
    WScript.Quit 1
End If
builtAt = files.GetFile(engine).DateLastModified
stale = files.FolderExists(root & "\target\pil-build-in-progress")
If Not stale Then stale = NewerSources(files.GetFolder(root & "\src"), builtAt)
If Not stale Then stale = NewerSources(files.GetFolder(root & "\isa"), builtAt)
If Not stale Then stale = NewerFile(root & "\Cargo.toml", builtAt)
If Not stale Then stale = NewerFile(root & "\Cargo.lock", builtAt)
If stale Then
    If Not checkOnly Then MsgBox "The runtime is being rebuilt or is older than the source. Wait for a successful PILbuild.cmd before opening the browser.", vbInformation, "PIL Retro - build required"
    WScript.Quit 1
End If
If checkOnly Then WScript.Quit 0
On Error Resume Next
runtimeFolder = files.BuildPath(files.GetSpecialFolder(2), "PIL-Retro-runtime")
If Not files.FolderExists(runtimeFolder) Then files.CreateFolder runtimeFolder
runtimeCopy = files.BuildPath(runtimeFolder, "pil-retro-" & files.GetBaseName(files.GetTempName()) & ".exe")
files.CopyFile engine, runtimeCopy, False
If Err.Number <> 0 Then
    MsgBox "Could not prepare the shared runtime copy: " & Err.Description, vbCritical, "Retro browser"
    WScript.Quit 1
End If
' Run the unchanged generic engine copy so Cargo can replace its build output.
command = Chr(34) & runtimeCopy & Chr(34) & " --dos-catalog " & Chr(34) & "images/FD14LITE.img::COMMAND.COM" & Chr(34) & " " & Chr(34) & catalog & Chr(34)
result = shell.Run(command, 0, True)
If Err.Number <> 0 Then
    MsgBox "Could not start the retro browser: " & Err.Description, vbCritical, "Retro browser"
ElseIf result <> 0 Then
    MsgBox "The retro browser exited with code " & result & ". Run target\release\pil.exe --dos-catalog images/FD14LITE.img::COMMAND.COM images/retro/catalog.tsv in a terminal for diagnostics.", vbCritical, "Retro browser"
End If
If files.FileExists(runtimeCopy) Then files.DeleteFile runtimeCopy, True

Function NewerFile(path, timestamp)
    NewerFile = False
    If files.FileExists(path) Then NewerFile = (files.GetFile(path).DateLastModified > timestamp)
End Function

Function NewerSources(folder, timestamp)
    Dim item, child
    NewerSources = False
    For Each item In folder.Files
        If item.DateLastModified > timestamp Then
            NewerSources = True
            Exit Function
        End If
    Next
    For Each child In folder.SubFolders
        If NewerSources(child, timestamp) Then
            NewerSources = True
            Exit Function
        End If
    Next
End Function
