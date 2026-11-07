from pathlib import Path
import subprocess
from PIL import Image

root = Path(__file__).resolve().parent.parent
launcher = root / "PILdos.vbs"
icon = root / "PILdos.ico"

if not launcher.exists():
    raise SystemExit(f"Missing launcher: {launcher}")

logo = Image.open(root / "PILdos.png").convert("RGBA")
logo.thumbnail((256, 256), Image.Resampling.LANCZOS)

canvas = Image.new("RGBA", (256, 256))
canvas.alpha_composite(
    logo, ((256 - logo.width) // 2, (256 - logo.height) // 2)
)
canvas.save(
    icon, format="ICO",
    sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)]
)

def quote(value):
    return "'" + str(value).replace("'", "''") + "'"

script = f"""
$desktop = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut((Join-Path $desktop 'PIL DOS.lnk'))
$link.TargetPath = {quote(launcher)}
$link.WorkingDirectory = {quote(root)}
$link.IconLocation = {quote(str(icon) + ',0')}
$link.Description = 'PIL DOS'
$link.Save()
"""
subprocess.run(
    ["powershell.exe", "-NoProfile", "-Command", script],
    check=True
)
print("Created PILdos.ico and the PIL DOS Desktop shortcut.")