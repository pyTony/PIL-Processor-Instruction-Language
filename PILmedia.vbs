' Native catalog launcher. Hide the engine console, not its interactive window.
Option Explicit
Dim shell, files, root, engine, catalog, shellImage, command, result, checkOnly, runtimeFolder, runtimeCopy, builtAt, stale
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
shell.CurrentDirectory = root
engine = root & "\target\release\pil.exe"
shellImage = root & "\images\FD14LITE.img"
checkOnly = False
If WScript.Arguments.Count > 0 Then checkOnly = (WScript.Arguments(0) = "--check")
If Not files.FileExists(engine) Then
    Fail "The runtime " & engine & " is missing. Build it with PILbuild.cmd or cargo build --release --locked.", "Retro browser"
End If
If Not files.FileExists(shellImage) Then
    Fail "The DOS shell image " & shellImage & " is missing. The browser starts COMMAND.COM from it.", "Retro browser"
End If
catalog = FindCatalog()
If catalog = "" Then
    Fail "No catalog.tsv found. Looked for " & root & "\images\retro\catalog.tsv and catalog.tsv in each folder directly below images\retro. Set PIL_RETRO_CATALOG to use another file; the format is in docs\DOS_INSTALL_MEDIA.md.", "Retro browser"
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
    MsgBox "The retro browser exited with code " & result & ". Run target\release\pil.exe --dos-catalog images/FD14LITE.img::COMMAND.COM " & Chr(34) & catalog & Chr(34) & " in a terminal for diagnostics.", vbCritical, "Retro browser"
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

Sub Fail(message, title)
    If Not checkOnly Then MsgBox message, vbCritical, title
    WScript.Quit 1
End Sub

' PIL_RETRO_CATALOG, else images\retro\catalog.tsv, else the first
' catalog.tsv one folder below images\retro (for example beside an
' extracted image). Deeper folders are not searched.
Function FindCatalog()
    Dim configured, retro, child
    FindCatalog = ""
    configured = shell.ExpandEnvironmentStrings("%PIL_RETRO_CATALOG%")
    If configured <> "%PIL_RETRO_CATALOG%" And configured <> "" Then
        If files.FileExists(configured) Then FindCatalog = files.GetAbsolutePathName(configured)
        If FindCatalog = "" Then Fail "PIL_RETRO_CATALOG names " & configured & ", which does not exist.", "Retro browser"
        Exit Function
    End If
    retro = root & "\images\retro"
    If files.FileExists(retro & "\catalog.tsv") Then
        FindCatalog = retro & "\catalog.tsv"
        Exit Function
    End If
    If Not files.FolderExists(retro) Then Exit Function
    For Each child In files.GetFolder(retro).SubFolders
        If files.FileExists(child.Path & "\catalog.tsv") Then
            FindCatalog = child.Path & "\catalog.tsv"
            Exit Function
        End If
    Next
End Function
