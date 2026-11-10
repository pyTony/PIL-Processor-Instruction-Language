"""Create a "PIL DOS" shortcut that opens the Dine DOS terminal.

Targets always use this script's checkout, independent of the current directory.
Setup is checked before writing shortcuts. After moving a checkout, rerun this
script to refresh the absolute paths.

Windows: uses the bundled PILdos.ico (Pillow is needed only if it is missing) and puts
"PIL DOS.lnk" on the Desktop, starting PILdos.vbs.

Linux and Android (Termux + Termux-x11): writes pildos.desktop, starting
PILdos.sh with PILdos.png as its icon, into ~/.local/share/applications
(the desktop's application menu) and onto the Desktop folder if there is one.
In Termux it also writes ~/.shortcuts/PIL DOS for the Termux:Widget widget,
which starts Termux-x11 first when it is not already running.
"""
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
png = root / "PILdos.png"


def validate_setup():
    """Catch an unbuilt/unconfigured checkout before installing a silent link."""
    launcher = root / ("PILdos.vbs" if sys.platform == "win32" else "PILdos.sh")
    for required in (launcher, root / "tools/dos_terminal.py", root / "tools/pil_debugger.py"):
        if not required.is_file():
            raise SystemExit(f"Missing runtime file: {required}")
    executable = "pil.exe" if sys.platform == "win32" else "pil"
    engines = [folder / executable for folder in (root, root / "tools", root / "target/release", root / "target/debug")]
    if not any(path.is_file() for path in engines):
        raise SystemExit(f"No PIL engine in {root}.\nRun cargo build --release --locked there, then rerun this creator.")
    explicit = os.environ.get("PILDOS_CONFIG")
    config = Path(explicit).expanduser() if explicit else root / "pildos.conf"
    if not config.is_absolute():
        config = root / config
    if explicit and not config.is_file():
        raise SystemExit(f"PILDOS_CONFIG does not exist: {config}")
    if not config.is_file():
        image = root / "images/FD14LITE.img"
        drive = root / (os.environ.get("PILDRIVE", "driveD") if sys.platform != "win32" else "DriveD")
        missing = [str(image)] if not image.is_file() else []
        if not drive.is_dir():
            missing.append(str(drive))
        if missing:
            raise SystemExit("Missing default DOS data:\n" + "\n".join(missing) +
                             "\nLink the existing directories into this checkout, or create pildos.conf with your data paths.")


def windows():
    launcher = root / "PILdos.vbs"
    icon = root / "PILdos.ico"
    if not launcher.exists():
        raise SystemExit(f"Missing launcher: {launcher}")

    if not icon.is_file():
        from PIL import Image

        logo = Image.open(png).convert("RGBA")
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
    print(f"Created PIL DOS Desktop shortcut.\nTarget: {launcher}\nStart in: {root}")


def desktop_folder(home):
    """The XDG Desktop folder, or None when there is none."""
    try:
        out = subprocess.run(["xdg-user-dir", "DESKTOP"], capture_output=True,
                             text=True, check=True).stdout.strip()
        folder = Path(out)
        if out and folder != home and folder.is_dir():
            return folder
    except (OSError, subprocess.CalledProcessError):
        pass
    folder = home / "Desktop"
    return folder if folder.is_dir() else None


def desktop_quote(value):
    """Quote an argument for the Exec key of a .desktop file."""
    text = str(value)
    for ch in ('\\', '"', '`', '$'):
        text = text.replace(ch, '\\' + ch)
    return '"' + text + '"'


def write_executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    path.chmod(0o755)
    print(f"Wrote {path}")


def unix():
    launcher = root / "PILdos.sh"
    if not launcher.exists():
        raise SystemExit(f"Missing launcher: {launcher}")
    launcher.chmod(0o755)
    home = Path.home()
    termux = "TERMUX_VERSION" in os.environ or "com.termux" in os.environ.get("PREFIX", "")

    entry = "\n".join([
        "[Desktop Entry]",
        "Type=Application",
        "Name=PIL DOS",
        "Comment=FreeDOS command prompt in the PIL DOS terminal",
        f"Exec={desktop_quote(launcher)}",
        f"Path={root}",
        f"Icon={png}",
        "Terminal=false",
        "Categories=System;Emulator;",
        "",
    ])
    write_executable(home / ".local/share/applications/pildos.desktop", entry)
    folder = desktop_folder(home)
    if folder:
        write_executable(folder / "pildos.desktop", entry)

    if termux:
        widget = "\n".join([
            "#!/data/data/com.termux/files/usr/bin/sh",
            "# PIL DOS for Termux:Widget: start Termux-x11 if needed, then the terminal.",
            "export DISPLAY=${DISPLAY:-:0}",
            "if ! pgrep -f termux.x11 >/dev/null 2>&1; then",
            "    termux-x11 \"$DISPLAY\" >/dev/null 2>&1 &",
            "    sleep 2",
            "fi",
            "am start --user 0 -n com.termux.x11/com.termux.x11.MainActivity >/dev/null 2>&1",
            f"exec {desktop_quote(launcher)} \"$@\"",
            "",
        ])
        write_executable(home / ".shortcuts" / "PIL DOS", widget)
    print("Created the PIL DOS shortcut.")


if __name__ == "__main__":
    validate_setup()
    if sys.platform == "win32":
        windows()
    else:
        unix()
