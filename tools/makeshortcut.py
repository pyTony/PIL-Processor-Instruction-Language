"""Create a "PIL DOS" shortcut that opens the PILdos setup screen.

The shortcut starts the launcher (PILdos.vbs or PILdos.sh), which runs the
Rust setup screen `pil --pildos`: it edits pildos.conf and F10 starts the
configured program. Only this one-time creator uses Python.

Targets always use this script's checkout, independent of the current directory.
Setup is checked before writing shortcuts. After moving a checkout, rerun this
script to refresh the absolute paths.

Windows: uses the bundled PILdos.ico (Pillow is needed only if it is missing) and puts
"PIL DOS.lnk" on the Desktop, starting PILdos.vbs.

Linux: writes pildos.desktop, starting PILdos.sh in a terminal with
PILdos.png as its icon, into ~/.local/share/applications (the desktop's
application menu) and onto the Desktop folder if there is one.
Termux: also writes ~/.shortcuts/PIL DOS for the Termux:Widget widget, which
opens the setup screen in a Termux session (no Termux-x11 needed).
"""
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
png = root / "PILdos.png"


def validate_setup():
    """Catch an unbuilt checkout before installing a silent link."""
    launcher = root / ("PILdos.vbs" if sys.platform == "win32" else "PILdos.sh")
    if not launcher.is_file():
        raise SystemExit(f"Missing launcher: {launcher}")
    executable = "pil.exe" if sys.platform == "win32" else "pil"
    explicit_engine = os.environ.get("PILDOS_ENGINE")
    engines = [Path(explicit_engine).expanduser()] if explicit_engine else [
        folder / executable for folder in (root, root / "target/release", root / "target/debug")]
    if not any(path.is_file() for path in engines):
        raise SystemExit(f"No PIL engine in {root}.\nRun cargo build --release --locked there, then rerun this creator.")
    explicit = os.environ.get("PILDOS_CONFIG")
    if explicit:
        config = Path(explicit).expanduser()
        if not config.is_absolute():
            config = root / config
        if not config.is_file():
            raise SystemExit(f"PILDOS_CONFIG does not exist: {config}")
    # Without pildos.conf the setup screen starts from pildos.conf.example.


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
        "Comment=PILdos setup screen; F10 starts the configured DOS program",
        f"Exec={desktop_quote(launcher)}",
        f"Path={root}",
        f"Icon={png}",
        "Terminal=true",
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
            "# PIL DOS for Termux:Widget: the PILdos setup screen in this session.",
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
