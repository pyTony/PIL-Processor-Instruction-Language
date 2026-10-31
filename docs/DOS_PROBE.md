# Executable renamed to pil — 31 October 2026

Tony chose `pil` as the executable's new name; `pil-proof` was the project's
old name. The crate is now `pil` (Cargo.toml and Cargo.lock), so builds
produce `target/release/pil` and `target/debug/pil` (`pil.exe` on Windows).
The build, debug, bench, compare and Mandelbrot scripts, the debugger and
DOS terminal engine lookup, the packaging and runtime export, CI and the
current documentation use the new name; the packaged folder is `pil/`.
Dated history entries keep the old name. A `pil-proof` binary left in an
existing target folder is no longer used by the scripts and can be deleted.

# AltGr characters in the terminal on Windows — 31 October 2026

Tony reported that `\` (AltGr + the `+` key on a Finnish layout) still did
not type in tools/dos_terminal.py on Windows. Windows sends AltGr as Ctrl and
Alt together, and the key event can then carry the Ctrl form of the
character (FS 1Ch for `\`, ESC for `{`, nothing for `@`), which DOS took as a
control key. With Ctrl and Alt both down, the terminal now types the
character the key's Tk name gives (`\ @ { } [ ] | $ ~ < > # ^ £ µ` and the grave accent); plain
Ctrl+`\` is still FS, and Alt with a letter, digit or F-key is still an Alt
code. On Windows the shift flags no longer show Ctrl+Alt while AltGr is
down. `--keylog` prints each key event (name, character, state) to the
console, for checking what a host keyboard sends. The Dine side was checked:
FreeCOM and BBC BASIC take `\` with Ctrl+Alt held. Not yet run on Windows.

# COM1 to a host terminal — 31 October 2026

Tony asked for the DOS COM port on a host ANSI terminal. A DOS session now
has COM1 at 3F8h, listed in the BIOS data area and equipment word:

- INT 14h for COM1: AH00 init, AH01 send, AH02 receive (a timeout at once
  when nothing has arrived, the BIOS answer without its wait), AH03 status.
- The 8250 registers at 3F8h-3FFh, polled: data, divisor latch, line and
  modem control, line and modem status, scratch. Enabling UART interrupts
  stops the run, as no IRQ 4 is delivered.
- DOS: handle 3 (AUX), and COM1 or AUX opened or created by name, write to
  the port and read what arrived (a read waits while nothing has).
  AX=4400h/4401h keep the AUX device information. After `CTTY COM1` the DOS
  console calls (AH01, 06, 07, 08, 0A, 0B, 0C input; 02, 06, 09 output) use
  the port; AH0A edits with Backspace. CON opens by name, so `CTTY CON`
  returns; setting the console's unchanged information is taken, raw mode
  on the console is not.
- Session protocol: replies carry `com1` (hex bytes sent since the last
  reply), and `com1 HEX` delivers bytes from the host.
- `tools/dos_terminal.py --com1 console` connects COM1 to the console the
  terminal was started from (keys typed there go to COM1; arrows and
  editing keys as VT100 sequences; Windows consoles get VT processing on),
  and `--com1 tcp:2323` to a localhost TCP port for telnet or PuTTY (Raw).

The probe keeps no serial port, as before.

Also in this change: AltGr characters (\ @ { } [ ] on a Finnish layout)
type again in the Tk terminal. Since the Alt keys change, AltGr (Ctrl+Alt on
Windows) was taken for Alt and its characters dropped; now only Alt with a
letter, digit or F-key sends an Alt code, and other keys type their
character.

Verification: unit tests for INT 14h, the polled 8250, AUX and COM1
handles and CTTY COM1 line input; GUI tests for the code page 437 helpers
and a TCP round trip; 422 Rust tests pass. With FD14LITE.img (local only):
`ECHO hello >COM1` reached the port; after `CTTY COM1`, VER and DIR ran over
the port and `CTTY CON` brought the shell back to the screen; through the Tk
terminal, `--com1 tcp:2323` and `--com1 console` (on a pty) both carried
VER, and ANSI escapes passed through unchanged.

# Finnish dates and times in the DOS terminal — 31 October 2026

Tony found the US date format confusing. Dine now has country 358
(Finland) next to 001 (USA, still the default), both in code page 437:
day.month.year, "." in dates and times, a 24-hour clock, "," for decimals,
a space for thousands, "EUR" after the amount and ";" as the list
separator, following the Finnish standard SFS 4175. INT 21h AH38 reads the
current country (AL=FFh and BX for codes over 254), gives either record on
request, and sets the country with DX=FFFFh; AH65 information records carry
the current country. The session takes `country 358`, and
`tools/dos_terminal.py --country 358` sends it at start.

Verification: unit test for the 358 record, setting and reading the country
and the AH65 record; GUI test for the session command; 420 Rust tests pass.
With FD14LITE.img (local only) and country 358, FreeCOM's `PROMPT $D $T`
showed "Sat 31.10.2026  3.16.42,44", DIR listed "46 256  02.04.25 14.22",
and DATE asked for "dd.mm.[cc]yy".

# FreeDOS EDIT, MEM and BBC BASIC run in the DOS terminal — 31 October 2026

Tony's runs stopped in three programs; each now runs in the terminal.

- EDIT: INT 33h AX=0 (mouse reset) answers AX=0 and BX=0, which means no
  mouse driver, and INT 21h AH=33h AL=00h-02h get and set the BREAK flag
  (kept, not acted on). EDIT opens a file, takes typing and saves it.
- Alt keys: the session takes `shift FLAGS` (BIOS 0040:0017 bits 0-3), the
  terminal sends Shift, Ctrl and Alt as they go down and up, and Alt with a
  letter, digit or F-key sends its BIOS Alt code. A shift change waits until
  the program has run 10 slices after the key before it, so Alt+F opens
  EDIT's File menu instead of opening and closing it.
- MEM: ports 70h/71h read the CMOS as an AT with 640K base memory, no
  extended memory, no diskette or hard disk and CGA, and its clock as
  INT 21h AH2A/AH2C give it (BCD, 24-hour). CMOS writes stop the run.
- BBC BASIC 4.82: it checksums its code up to its stack segment, 10 bytes
  past the size its MZ header declares. DOS loads whole 512-byte pages to the
  end of the file, and Dine now does the same (first program and EXEC);
  bytes past the last page are still not loaded.

ZIP.EXE is not in this change: it runs the PMODE/W extender, which switches
to protected mode itself (LIDT, LGDT, CR0).

Verification: 419 Rust tests and the GUI tests pass, with new tests for the
mouse reset, BREAK flag, CMOS reads and refusals, whole-page MZ loading,
Alt codes and the `shift` command. With FD14LITE.img (local only): EDIT
opened D:\NOTE.TXT, took text, saved it through the File menu, and Alt+F
opened the menu in the Tk terminal under Xvfb; MEM printed its memory table;
BBCBASIC.EXE (Tony's copy, not in the repo) printed its banner, `PRINT 6*7`
gave 42 and a FOR loop printed 1 2 3.

# Startup command line for the DOS terminal — 30 October 2026

`--dos-session` takes `--args COMMAND_LINE` and puts it in the PSP command
tail at 80h as COMMAND.COM passes one (count, leading space, text, CR;
printable ASCII, at most 125 characters). `tools/dos_terminal.py --args`
passes it through, so `--args "/K D:\START.BAT"` has FreeCOM run a batch file
from the mounted folder and then stay at the prompt. Without `--args` the
tail stays empty, as before.

Verification: unit test for the tail bytes and limits; GUI test where a COM
writes its tail back; 418 Rust tests pass. With FD14LITE.img (local only),
`--mount D --args "/K D:\START.BAT"` ran START.BAT (ECHO, VER, PROMPT $T $P$G)
before the first prompt, and DIR /W worked afterwards.

# Time zone option for the DOS terminal clock — 30 October 2026

On Tony's Windows machine the terminal's DOS clock showed UTC: the local
offset Python reported there was 0. `tools/dos_terminal.py` now takes
`--utc-offset HOURS` (2, -5, 5.5 or +02:00, at most 14 hours) to set the time
zone of the DOS clock, reads the default offset with
`datetime.now().astimezone()`, and shows the zone in use on the status line
(`clock UTC+02:00`).

Verification: GUI tests pass (offset parsing); under Xvfb, `--utc-offset 2`
made FreeCOM's `PROMPT $T` show 01:02 at 23:02 UTC, and the status line read
`clock UTC+02:00`.

# Visible text selection in the DOS terminal — 30 October 2026

A mouse selection in the Tk terminal was made but never shown: the colour
tags that paint each cell's background lay above Tk's selection tag. The
selection tag is now raised above them, with a blue highlight that stays
visible while the window does not have the focus, so text can be picked
for Ctrl+Shift+C.

Verification: a selection dragged under Xvfb shows highlighted in a
screenshot; the GUI tests pass.

# Host clock and copy/paste in the DOS terminal — 30 October 2026

The Tk terminal now runs DOS time on this computer's clock, so FreeCOM's
`PROMPT $D $T` and `TIME` show the local date and time and can time a
command. A new session command, `clock OFFSET` (seconds east of UTC),
switches the clock to host time: INT 1Ah AH=00h returns the local time of
day at 1193180/65536 ticks a second (rollover set when midnight passed),
AH2C follows it, and AH2A returns the local date and day of the week. The
terminal sends it at start with the local offset, daylight saving included.
Without it, as in the probe and the tests, the clock stays deterministic from
midnight on 1 January 1980.

The terminal also copies and pastes: Ctrl+Shift+C copies the selection, or
the whole screen with trailing blanks dropped; Ctrl+Shift+V or Shift+Insert
types the clipboard as keystrokes (line ends as Enter, tabs as Tab, other
characters through code page 437, those it lacks left out). Both are on the
right-click menu, and a selection survives screen redraws. Plain Ctrl+C and
Ctrl+V still reach DOS.

Limitations: setting the date or time (AH2B, AH2D) still stops the run; the
BIOS tick count at 0040:006C changes only when a program asks for the time.

Verification: 417 Rust tests passed on Linux (one new host-clock test); the
GUI tests pass (paste keystrokes and the `clock` command). FreeCOM 0.86 in
the terminal under Xvfb showed `PROMPT $D $T` as the local date and time,
and a pasted `ECHO pasted äö` and `PROMPT $T $P$G` ran; Ctrl+Shift+C copied
the screen.

# MANDBBC.COM: the BBC BASIC Mandelbrot — 30 October 2026

`programs/dos/MANDBBC.COM` draws the classic BBC BASIC benchmark Mandelbrot:
X from -49 to 29 (C = X*2.29/50), Y from -12 to 12 (D = Y*4.16/50), z
starting at c, escape at |z|^2 >= 5, at most 16 iterations, and the shades
`.,'~=+:;*%&$OXB#@`. It uses 32-bit integers with 20 fraction bits (386
instructions in real mode). A Python model of fewer fraction bits differs
from the floating-point program in 1 to 95 of the 1975 cells; with 20 bits
none differ, so MANDBBC prints the same characters as BBC BASIC with
floating point.

Verification: 416 Rust tests passed on Linux; the new test compares all 25
rows of MANDBBC's output in Dine with the rows the floating-point program
gives (computed in Python from the BASIC listing). FreeCOM ran `MANDBBC`
from D:.

# Z: memory drive — 30 October 2026

Every `--dos-session` now has Z:, a drive held in memory, as DOSBox has its
own Z:. It starts empty, takes the same create, write, delete, rename and
directory services as the host folder, and is gone when the session ends.
The environment gains `TEMP=Z:\`, so FreeCOM's pipes work from C: (the
read-only image) without a SET first. Z: holds 32 MiB in 2 KiB clusters, and
AH36 reports its real free space; when a write does not fit, nothing is
written and AX=0, as DOS reports a full disk. AH0E now reports drives up to
Z:. The probe (no drives) is unchanged.

Verification: 415 Rust tests passed on Linux (one new memory-drive test).
FreeCOM 0.86 ran `SET` (shows TEMP=Z:\), `DIR | D:MORE` from C:, and on Z:
`ECHO > A.TXT`, `MD`, `COPY`, `REN`, `DIR /S` (33,550,336 bytes free),
`TYPE`, `DEL` and `RD`.

# PC speaker ports — 30 October 2026

FreeCOM's beep (the `BEEP` command, and a key the line editor does not take,
such as Ctrl-V) stopped at IN AL,DX (opcode ECh). Dine now answers byte IN
and OUT on the speaker ports on the host, as it answers INT services
(`src/dos_ports.rs`): PIT channel 2 takes its mode at 43h and its count at
42h, and port 61h bits 0 and 1 switch the speaker. Reading 61h returns the
bits last written, with bit 4 toggling as the refresh bit does. The session
reply reports `beeps` (times the speaker was switched on) and `tone` (Hz, 0
when off), and the Tk terminal rings the host bell for each new beep.

Limitations: the PIT does not count, so reading 42h, other PIT commands, word
port accesses and every other port still stop the run; bit 5 of port 61h
(channel 2 output) reads 0.

Verification: 414 Rust tests passed on Linux (one new test for Watcom's
sound and nosound sequences and for the ports that still stop). FreeCOM 0.86
ran `BEEP` and a Ctrl-V at the prompt, then `ECHO after`, with `beeps` 1 and
then 2.

# Writable host folder: pipes, redirection and file commands — 30 October 2026

The `--mount` folder (D: after an image) can now be written; the image stays
read-only. DOS programs create, write, delete and rename files and make and
remove directories there, and the changes reach the host folder itself.
FreeCOM's `DIR > OUT.TXT`, `ECHO ... >> FILE`, `DIR | MORE`, `COPY`, `DEL`,
`REN`, `MD` and `RD` work on that drive.

- AH3C, AH5A and AH5B create files; AH3D opens host files for writing (modes
  1 and 2); AH40 writes through to the host file at the file position, and
  CX=0 truncates or extends the file there.
- AH41 deletes, AH56 renames (same drive only, error 11h otherwise), AH39 and
  AH3A make and remove directories (error 10h for a current directory).
- AH57 gets and sets a file's time and date; AH60 (truename) returns the full
  upper-case path.
- Opens and creates take the lowest free handle, as DOS does, so FreeCOM's
  close-then-open redirection lands on handle 0 or 1. AH02, AH06 and AH09
  output goes to the file when handle 1 was redirected.
- AH4F goes on after the host name it found last, so `DEL *.*` does not skip
  files when the directory changes between searches.
- New host files take the upper-case 8.3 name (`OUT.TXT`).

FreeCOM puts its pipe file in TEMP, TMP or the current directory. From the
read-only C:, use `SET TEMP=D:\` first, or FreeCOM reports "Can not create
temporary file".

Limitations: AH43 AL=01h (set attributes) is still access denied, AH36 still
reports no free space (the host's free space is not read), attributes given
to a create are not kept, two opens of one file do not see each other's
writes until reopened, and redirected standard input for AH01/06/07/08/0A is
not modelled.

Verification: 413 Rust tests passed on Linux (one new test for the writable
folder; the image test now checks every write is refused). FreeCOM 0.86 from
FD14LITE ran `DIR > OUT.TXT`, `TYPE OUT.TXT`, `ECHO >>`, `DIR | MORE` with
FreeDOS MORE, `COPY`, `REN`, `MD`, `COPY *.TXT SUB`, `DEL SUB\*.*`, `RD`,
`DEL *.TXT` and a refused `ECHO x > C:\X.TXT`, with the results checked in
the host folder.

# AH33 AL=05h boot drive — 30 October 2026

`DIR | MORE` stopped at INT 21h AX=3305h. FreeCOM asks for the boot drive
when no TEMP, TMP or current directory can take a temporary file, which is
always the case while every drive is read-only. Dine now reports its first
drive (C: with an image or a mounted folder, A: without). Pipes still cannot
work until some drive can be written: FreeCOM falls back to running the left
side to the screen and the right side reading the keyboard.

Verification: 412 Rust tests passed on Linux.

# IRET in real mode — 30 October 2026

`MORE` stopped at opcode CFh (IRET) in its own code before printing anything.
IRET and IRETD now run in real mode, in `isa/i386/iret.pil`: the whole IP, CS
and FLAGS frame is checked against SS first, then the far branch, SP and the
flags POPF can write (bits 1, 3, 5 and 15 stay fixed; RF and VM are kept) are
committed together. Protected-mode IRET still stops.

Verification: 411 Rust tests passed on Linux; 252 native verifier cases (5 new:
IRET and IRETD with 16- and 32-bit stacks, and a frame fault); DOSBox-X
2024.03.01 matched PIL on all 522 comparison cases (3 new IRET cases).

# INT 10h EGA/VGA functions on the CGA BIOS, and handle duplication — 30 October 2026

`SETLANG SU` in FD14LITE stopped at INT 10h AH=11h (character generator).
Dine's video BIOS is CGA class on an AT BIOS, which returns from AH=10h-12h
(reserved entries) and from functions past its table without changing
anything. AH=10h, 11h, 1Ah, 1Bh and 1Ch now do the same, as AH=12h and 4Fh
already did. Programs that check AX=1A00h or BL=10h see no VGA and no EGA.

`MORE SETLANG.BAT` stopped at INT 21h AH=45h. AH45 (duplicate a handle into
the lowest free one) and AH46 (force CX to refer to BX's file, closing CX
first) now work. Handles are resolved through their JFT entry, so duplicates
share one file position, and a file stays open until no handle in the running
program or its waiting parents refers to it. Writing to a file handle returns
access denied (5), since every file is opened read-only.

The terminal (`tools/dos_terminal.py`) now passes characters beyond ASCII
that the host keyboard layout produced, such as ä, ö and å, in code page 437,
with the scan code of their key in the Nordic layouts. Characters 437 lacks
(such as €) are still dropped. Since the host already applies the layout, no
DOS KEYB driver is needed. The screen now draws all 256 characters with their
IBM PC glyphs, including ☺ ♥ ♪ ► ↑ for 01h-1Fh and ⌂ for 7Fh, which Python's
cp437 codec leaves as control characters; § and ¶ can be typed as 15h/14h.

Verification: 410 Rust tests passed on Linux.

# EXEC: programs start from FreeCOM, plus FACTOR and MANDEL — 30 October 2026

Step 4 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot). In
Tony's screenshots after #59, programs typed at the FreeCOM prompt stopped at
AH29 just before EXEC, and DATE/TIME stopped reading CON through AH3F.

- INT 21 AH4B AL=00h (load and execute), after FreeDOS task.c:
  - COM files and MZ EXE files, with relocations applied at the load segment.
    The program comes from any Dine drive. A missing file gives error 2 or 3,
    a directory error 5, and an unusable header error 11.
  - The child gets its own environment block (the given one or the parent's,
    then the count word and its full path) and program block. A COM file takes
    the largest free block; an EXE takes what fits between its minimum and
    maximum. Error 8 is returned when memory is short.
  - The PSP carries INT 20h, the block end, the parent's return address and
    INT 23h/24h vectors, the parent PSP, a copy of the parent's handles, the
    environment, the 20-entry handle table, the DOS version word, the INT 21h
    RETF stub, both FCBs and the command tail.
  - When the child ends (AH4C, INT 20h, AH31), its memory is freed (all
    but the resident block for AH31) and the files it opened are closed. The
    parent continues after its INT 21h with CF=0. Programs can nest 8 deep.
  - The PSP is no longer fixed at 1000h: AH51/62, AH48 ownership, the handle
    table and termination all follow the running program.
  - AL=01h and AL=03h (load without running, overlays) still stop the run.
- AH4D returns the last child's exit code and how it ended (AH=00h normal,
  03h resident) once, then 0.
- AH29 parses a file name into an FCB as FreeDOS FcbParseFname does: the four
  AL option bits, drive letters (FFh for a drive that does not exist), `*` and
  `?`, `.` and `..`, and DS:SI left after the name.
- AH3F on handles 0-2 while they are still CON reads a cooked line: the same
  editor as AH0A (127 characters), CR echoes LF, and the line with CR LF is
  handed out over as many reads as the program asks for.
- `programs/dos/` has two original demo programs with their NASM sources:
  `FACTOR.COM [n]` prints 0! to n! as exact decimals, and `MANDEL.COM` draws
  the Mandelbrot set in text characters with 8.8 fixed point.

From the FreeCOM 0.86 prompt this now works: running COM and EXE programs from
C: and D:, `IF ERRORLEVEL`, a nested `COMMAND` and `EXIT` back to the first
one, and typing a new date at the DATE prompt (setting it, AH2B, still stops).

  `python tools/dos_terminal.py --mount programs/dos images/FD14LITE.img::COMMAND.COM`

Verification: 408 Rust tests passed on Linux, including EXEC of a COM and a
relocated EXE with their return codes, AH29 cases from FreeDOS's rules, a CON
read split over two calls, and the output of both demo programs. FreeCOM 0.86
was driven through HELLO, an EXE, FACTOR 30, MANDEL, nested COMMAND and DATE
on a test FAT16 image plus a host folder. FD14LITE itself is not in CI.

# Drives C: and D:, DIR in FreeCOM, IDIV and country information — 30 October 2026

Step 3 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot). FreeDOS
report 47 (dev after #56 and the DOS data area): 69 Exit, and COMMAND.COM got
past AH52 to AH43, the most common stop (16 programs).

- Dine has drives now, all read-only:
  - `--dos-session IMAGE.img::PROGRAM` makes the FAT16 image drive C:, with its
    volume label, attributes, dates and directory tree. COMSPEC and the program
    path point at C:.
  - `--mount DIR` adds a host folder as the next drive (D:, or C: without an
    image). Names that are not 8.3 get BASIS~N aliases per directory
    (`a-long-file-name.txt` is `A-LONG~1.TXT`); dates come from the host
    modification time in UTC. Symbolic links are not followed.
  - With neither, A: is an empty drive, as before.
- DOS services on those drives: AH4E/4F find (attribute filtering, `.` and `..`
  below the root, label-only searches), AH3B CD, AH47 current directory, AH0E
  and AH19 drive selection (LASTDRIVE E:), AH36 (an image's own geometry; no
  cluster is free), AH43 get attributes, and AH3D/3F/42/3E reading any file.
  Writes (AH39/3A/3C/41/56/5A/5B, AH43 set, AH3D for writing) return access
  denied. AH44/00 gives files their drive and CON bit 6 (not at end of input),
  as FreeDOS opens devices. AX=6900h and AH73 are invalid functions.
- Country 001, code page 437: AH38, AX=6501h, the 6502/04/05/06/07 table
  pointers, AH65 upper-case calls (20h-23h, A0h-A2h) and a far case-map
  routine in Dine's data segment that programs call.
- AH58 accepts the 4xh (upper memory only) strategies and allocates low, as
  FreeDOS does without UMBs. INT 2F AX=AE00h and B700h find no installable
  command and no APPEND.
- IDIV (F6/F7 /7): signed division through magnitudes, in both backends, with
  7 native verifier cases and 15 DOSBox-X comparison cases.

FreeCOM 0.86 now runs interactively: the `C:\>` prompt, DIR (volume label, 8.3
names, `<DIR>`, sizes with thousands separators, totals), CD, switching
between C: and D:, TYPE and VER.

  `python tools/dos_terminal.py --mount dosfiles images/FD14LITE.img::COMMAND.COM`

Verification: 404 Rust tests passed on Linux, 247 native verifier cases, and
DOSBox-X 2024.03.01 matched PIL on all 519 comparison cases. FreeCOM 0.86 was
driven through DIR, CD, C:/D: switching and TYPE on a test FAT16 image plus a
host folder. FD14LITE itself is not in CI.

# Interactive DOS session and text-mode terminal — 30 October 2026

Step 2 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot):

- `pil-proof --dos-session PROGRAM` loads a .COM/.EXE, or IMAGE.img::INNER
  from a container, and serves one command per stdin line with one JSON reply
  per line:
  - `run N` executes up to N steps (1..1000000). The run pauses on the step
    budget and when the program waits for a key, and ends at exit or at the
    first unmodelled service, encoding, fault or boundary.
  - `key ASCII SCAN` puts one keystroke in the BIOS keyboard ring.
  - `snapshot` replies without running, and `quit` ends the session.
  - Every reply carries the status, the active B800 text page (character and
    attribute bytes as hex), its width, the BIOS cursor, CS:IP, the step
    count and the program's DOS output for that slice.
- tools/dos_terminal.py is a Tk window for the session. It draws the 80x25 page
  in the 16 CGA colours with the cursor, maps keys to BIOS ASCII and scan codes
  (US layout; arrows, F1-F10, Home/End, PgUp/PgDn, Ins/Del, Ctrl+letter), and
  shows the status, step count and CS:IP. It only displays: execution, Dine
  services and screen contents all come from the engine.

  `python tools/dos_terminal.py images/FD14LITE.img::COMMAND.COM`

Verification: 398 Rust tests passed on Linux (a new session test covers run,
the 15-key ring, exit and malformed commands). tests/gui/test_dos_terminal.py
covers key mapping, CP437 decoding and a live session that waits for keys,
echoes them and exits. The window was also driven under Xvfb with typed keys.

# DOS data area: list of lists and InDOS — 30 October 2026

FreeDOS report 46 (dev after #56, 100,000 steps): 67 Exit, 48 Budget, 33
Service, 3 MissingEncoding. FreeCOM now loads its strings through the
environment block and its own file. It then stops at INT 21h AH=52h, as do 20
other programs. This round:

- Dine's DOS data segment is 0070h. INT 21h AH=52h returns ES:BX = 0070:0026,
  the list of lists, with the first MCB segment in the word before it. That is
  the only field FreeCOM reads (lib/get1mcb.c), and the MCB chain it leads to
  is the real one: environment, program, end. The NUL device header ends the
  device chain and sectors are 512 bytes. Structures Dine does not model (DPB,
  SFT, CLOCK$, CON, buffers, CDS, FCBs) are FFFF:FFFF, and there are no block
  devices or drives yet.
- INT 21h AH=34h returns ES:BX = 0070:0001, the InDOS byte, which is 0. The
  critical-error byte before it is also 0.

Verification: 397 Rust tests passed on Linux. Not yet rerun on the FreeDOS
image.

# SHLD/SHRD, environment block, own-file reads and a growable handle table — 30 October 2026

FreeDOS report 45 (dev after #55, 100,000 steps): 65 Exit, 16 WaitingForInput,
45 Budget, 17 Service, 8 MissingEncoding. The BTC probe is gone. This round:

- SHLD and SHRD (0F A4/A5/AC/AD) by immediate or CL, on registers and memory,
  in isa/i386/shld.pil and shrd.pil. The count is masked to five bits and a
  zero count changes no flags. CF is the last bit shifted out, OF is defined
  only for a count of one, and AF is undefined. A 16-bit count above 16 leaves
  the result undefined on the 386, so it is rejected instead of guessed. All
  four CWSDPMI programs and FDNPKG stopped on `SHLD DX,AX,CL`.
- Environment block: PSP:2Ch points at a 16-paragraph block in its own MCB,
  owned by the program (so AH49 can free it), holding `COMSPEC=A:\COMMAND.COM`,
  the end-of-list NUL, the count word 1 and the program path `A:\NAME.EXT`.
- The program's own file is the one file the probe can open. AH3D with a path
  ending in the program's name opens it read-only on the lowest free handle;
  AH3F reads from it, AH42 seeks (origins 0, 1, 2; DX:AX is the new position)
  and AH3E closes it. Opening it for writing, a second open, and seeking before
  its start stop. Other paths still return file-not-found. Directories are not
  modelled. This was salvaged from the `feature/command-interactive` branch.
- INT 21h AH=67h follows FreeDOS SetJFTSize. PSP:32h/34h now hold the table
  size (20) and a far pointer to PSP:18h. A count that fits updates PSP:32h; a
  larger one allocates a block with AH48 rules, copies the entries, fills the
  rest with FFh and points PSP:34h at it. Without free memory it returns CF=1,
  AX=8. Handle opens and closes go through the pointer. SWSUBST asked for 50.
- DOSBox-X CPU suite is table-driven: 504 cases in a 31,169-byte COM (was
  448 cases in 64,398 bytes, near the FF00h limit). 16-bit SHLD-by-one OF is
  not compared, because DOSBox-X takes it from the source word.

Verification: 396 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 240 cases (new: double_shift with
CL and immediate counts on registers and memory; double_shift_reject for a
16-bit count of 17). DOSBox-X 2024.03.01 headless: CPU suite 504/504, the
negative control fails case 1 in both engines, console 9/9. The Python tool
checks and GUI tests pass. Not yet rerun on the FreeDOS image.

# BT family, true DOS version, default drive and handle count — 30 October 2026

FreeDOS report 44 (dev after #54, 100,000 steps): 63 programs exit and 16
reach a keyboard prompt. This round:

- BT, BTS, BTR and BTC (0F A3/AB/B3/BB) on registers, and the 0F BA /4-/7
  immediate forms on registers and memory, in isa/i386/bt.pil. The bit offset
  is taken modulo the operand width, CF gets the selected bit, and OF, SF, ZF,
  AF and PF are undefined. Register-offset memory forms (bit strings past the
  operand) are still a decode stop. Five programs stopped on `BTC EAX,ESI`, the
  EFLAGS.AC probe that tells a 386 from a 486.
- INT 21h AX=3306h (true version) returns BX=1606h (6.22, the same profile as
  AH30), DL=0 (revision) and DH=0 (not in ROM or the HMA), as FreeDOS
  inthndlr.c does with its own version. Other AH33 subfunctions still stop.
  DEVLOAD stopped here.
- INT 21h AH=19h returns AL=0, drive A:, matching the single floppy in the
  INT 11h equipment word. This is a profile choice; there is no filesystem.
  ASSIGN stopped here.
- INT 21h AH=67h (set handle count) succeeds for BX up to 20, the PSP handle
  table Dine already has. Larger requests stop, because Dine does not move the
  table. SWSUBST stopped here.
- INT 2Fh AX=0100h (PRINT) and AX=B710h (APPEND) installation checks reach
  the kernel default and return AX unchanged.

Verification: 393 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 238 cases (new: bit_test with the
report's BTC EAX,ESI, register and immediate-memory forms; bit_test_readonly
faults on a read-only BTS). DOSBox-X 2024.03.01 headless: CPU suite 448/448
(48 BT-family cases new, COM 64,398 bytes), the negative control fails case 1
in both engines as intended. Not yet rerun on the FreeDOS image.

# One-operand IMUL, XLAT, DOS time and allocation strategies — 30 October 2026

FreeDOS report 42 (dev after #53, 10,000 steps): 84 Budget, 47 Exit, 15 Service,
5 MissingEncoding, and no Fault or Monitor stops. VDELAY, VPAUSE, DISKCOPY and
REPLACE got past their report-41 stops. This round:

- One-operand IMUL (F6 /5, F7 /5) at 8/16/32 bits, register or memory: the
  signed product goes to AX, DX:AX or EDX:EAX, and CF=OF=1 when it does not
  fit the sign-extended low half (SF, ZF, AF, PF undefined). DISKCOPY stopped
  on `IMUL DX`.
- XLAT (D7) in isa/i386/xlat.pil: AL = seg:[(E)BX+AL], DS unless overridden,
  BX+AL wrapping at 16 bits. GREP stopped on it.
- INT 21h AH=2Ch reads the BIOS tick count (one INT 1Ah read, so the clock
  advances one tick) and scales it to hours, minutes, seconds and hundredths
  by 65536*5/59659, the scaling the FreeDOS clock driver uses.
- INT 21h AH=58h/01h now also accepts best fit (1) and last fit (2), with or
  without the 80h high-memory bit (no UMBs, so 8xh allocates low). AH=48h
  follows the strategy: best fit takes the smallest block that fits, last fit
  takes the top of the last block that fits. Upper-only strategies (4xh)
  still stop. SWSUBST stopped on last fit.
- INT 2Fh AX=0600h (ASSIGN installation check) returns AX unchanged.
- INT 15h AH=E8h (E801h/E820h memory maps) returns CF=1, AH=86h, like the
  other functions newer than this AT-class BIOS.
- DOSBox-X CPU suite: each case now calls one shared routine to print
  PASS/FAIL, so the COM shrank from 64,183 to 57,402 bytes with 400 cases.

Verification: 390 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 236 cases (new: imul1 with
8/16/32-bit register and memory forms, imul1 past the segment end, XLAT with
BX, EBX and ES). DOSBox-X 2024.03.01 headless: CPU suite 400/400 (6 one-operand
IMUL, 1 XLAT new), the negative control fails case 1 in both engines as
intended, and console 9/9. Not yet rerun on the FreeDOS image.

# Far JMP, TSR exit, date, BIOS identity bytes and HLT idle — 30 October 2026

FreeDOS report 41 (dev after #52, 10,000 steps): 81 Budget, 43 Exit, 19 Service,
4 MissingEncoding, 2 Fault, 2 Monitor. This round:

- Far JMP: EA ptr16:16/32 and FF /5 m16:16/32 (memory only) in
  isa/i386/jmp_far.pil, real mode, with the pointer read whole before the
  transfer. Report 41 stopped GREP on `JMP FAR [mem]`.
- INT 21h AH=31h ends the run as a TSR exit: the report shows Exit(AL) and a
  RESIDENT line with the paragraphs kept (at least 6, as in FreeDOS). No
  resident code runs afterwards.
- INT 21h AH=2Ah returns 1 January 1980, a Tuesday: the DOS default with no
  clock driver (FreeDOS systime.c computes the weekday as (days+2) mod 7).
- INT 21h AH=0Dh (disk reset) has no buffers to flush.
- INT 21h AH=58h: AL=00h returns the strategy; AL=01h accepts 00h and 80h
  (first fit; without UMBs 80h allocates low), rejects BL>82h or a fit above
  2 with error 1 as FreeDOS does, and stops on best/last fit, which the
  first-fit arena does not model. AL=02h returns link state 0; AL=03h fails
  with error 1 because there is no UMB chain.
- INT 21h AH=71h (long file names) returns AL=00h, CF=1 (AX=7100h), as
  FreeDOS does without an LFN API.
- INT 15h AH=88h returns AX=0: no memory above 1 MB.

- BIOS area (Tony's decision): C000:0000-F000:FFFF now reads as an empty bus
  (FFh, so no option-ROM 55AAh signature) and is read-only. The top 16 bytes
  identify a PC/AT-class BIOS: the reset jump, the date 01/10/84 at F000:FFF5
  and model byte FCh at F000:FFFE. No BIOS code is present. DISKCOPY and
  REPLACE faulted reading F000:FFEA, where they look for "COMPAQ"; they now
  read FFh. RAM grows from 768 KB to 1 MB to hold it.
- HLT (Tony's decision): with IF=1, as DOS starts programs, HLT idles until
  the next timer tick: one tick passes (seen by INT 1Ah) and execution
  continues. VDELAY and VPAUSE idle this way. With IF=0, HLT never resumes and
  stays a monitor stop. DOS unit fixtures that use HLT as an end marker now
  clear IF first.

Verification: 387 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 233 cases (new: far JMP immediate,
memory 16 and 32, pointer past the segment end, real-mode rejection). DOSBox-X
2024.03.01 headless: CPU suite 393/393 (2 far JMP new; the COM is 64,183 bytes,
close to the 64 KB limit) and console 9/9 in both engines. The date and DOS
memory answers are not compared with DOSBox-X, which uses the host clock and
has UMBs and XMS. Not yet rerun on the FreeDOS image.

# LDS, ENTER and absent-extension answers — 30 October 2026

FreeDOS report 40 (dev after #51, 10,000 steps): 79 Budget, 38 Exit, 28 Service,
6 MissingEncoding, down from 25 MissingEncoding in report 39. This round takes
the integer encodings and the services with a documented "not present" answer:

- LDS (C5) shares isa/i386/les.pil, which now takes the destination segment
  register. Report 40 stopped LBACACHE and RECOVER on it.
- ENTER size,0 (C8) in isa/i386/enter.pil: push BP/EBP, set BP to the pushed
  slot, reserve the frame. A non-zero nesting level is still a decode stop.
- INT 21h AH=40h writes CX bytes from DS:DX to the CON handles 0..2, with the
  same screen and capture path as AH=09h, and returns AX=CX. A closed handle
  returns error 6; AUX/PRN (3, 4) still stop as not modelled.
- INT 21h AH=51h returns the PSP in BX, like AH=62h (also added).
- INT 2Fh installation checks AX=1100h, 1600h, 1680h, 1687h, 1A00h and 4300h
  return AX unchanged, which is what the FreeDOS kernel's default multiplex
  handler does (kernel/int2f.asm, inthndlr.c) when no redirector, Windows,
  DPMI host, ANSI.SYS or XMS driver is loaded. AX=4310h (XMS entry point) is
  still a stop, because a caller should only ask after 4300h says installed.
- INT 10h AH=12h and AH=4Fh: the video BIOS is CGA-class, so these calls are
  ignored (BL=10h stays 10h: no EGA; AL is not 4Fh: no VBE).
- INT 15h AH=C0h returns CF=1, AH=86h (no configuration table), like AH=C2h.

Verification: 384 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 228 cases (new: lds16, lds32,
enter16, enter32, ENTER on a read-only stack slot). DOSBox-X 2024.03.01
headless: CPU suite 391/391 (2 LDS, 2 ENTER new) and console 9/9 (AH40 to
STDOUT, AH51/AH62) in both engines. INT 2Fh and INT 10h AH=12h are not compared
with DOSBox-X, which has its own XMS and a VGA BIOS. Not yet rerun on the
FreeDOS image.

# RCL/RCR, DAA/DAS, PUSHA/POPA and indirect JMP — 30 October 2026

The rest of the integer encodings that stop programs in FreeDOS reports 38 and 39:

- RCL and RCR (group 2 /2 and /3: D0-D3, C0/C1) at 8/16/32 bits, register or
  memory, in isa/i386/rcl.pil and rcr.pil. The count is masked to five bits
  and reduced modulo width+1 (CF is the extra ring bit). Flags change only for
  a non-zero masked count; OF only for a count of one (RCL: result MSB XOR new
  CF; RCR: input MSB XOR incoming CF), as the existing ROL/ROR do.
- DAA (27) and DAS (2F) from the Intel SDM pseudocode; OF is undefined.
- PUSHA/PUSHAD (60) and POPA/POPAD (61) on a 16- or 32-bit stack. PUSHA stores
  the original SP/ESP in the fifth slot; POPA skips that slot. PIL now allows
  up to eight staged stores per instruction (was two), so all eight PUSHA
  stores are checked before any commits: a protected or out-of-segment slot
  leaves registers and RAM unchanged.
- Near indirect JMP r/m16 and r/m32 (FF /4) from a register or memory, in
  isa/i386/jmp_indirect.pil. The new offset is zero-extended and checked
  against CS.limit. Report 39 stopped DISKCOPY, RECOVER, REPLACE, SHARE and
  UNFORMAT on `JMP [mem]` jump tables.

The DOSBox-X CPU suite puts identical strings in the COM once, so the 387 cases
still fit in one 64 KB COM.

Verification: 382 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 223 cases (new: RCL/RCR mix, RCL
on read-only memory, DAA/DAS chain, PUSHA/POPA 16 and 32, PUSHA on a read-only
stack slot, POPA at the top of the stack, indirect JMP register/memory/32-bit,
indirect JMP past CS.limit). DOSBox-X 2024.03.01 headless: CPU suite 387/387
(24 RCL/RCR, 10 DAA/DAS, 4 PUSHA/POPA, 3 indirect JMP new) and console 8/8 in
both engines. Not yet rerun on the FreeDOS image.

# LOOP family, unsigned MUL and INT15 AH=C2 — 30 October 2026

FreeDOS probe report 38 (current dev, 10,000 steps) stopped 16 programs on
missing encodings and 12 CTMOUSE builds on INT15 AH=C2. This adds:

- LOOP, LOOPE/LOOPZ and LOOPNE/LOOPNZ (E2, E1, E0) in isa/i386/loop.pil. The
  address size picks CX or ECX; the counter is decremented and written whether
  or not the branch is taken; flags are preserved. A taken target outside CS
  faults before the counter is written.
- Unsigned MUL F6/F7 /4 at 8/16/32 bits, register or memory: AX, DX:AX or
  EDX:EAX receive the product; CF=OF=1 when the upper half is non-zero;
  SF/ZF/AF/PF are undefined.
- INT15 AH=C2 (PS/2 pointing device) answers CF=1, AH=86h "function not
  supported", as a BIOS without PS/2 mouse support does (SeaBIOS behaviour).
  No mouse is modelled; Dine does not pretend one exists.

Verification: 380 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 214 cases (seven new MUL/LOOP
cases, including a memory-operand fault and a taken-target fault). The DOSBox-X
CPU comparison grows to 346 cases (12 MUL, 6 LOOP) and passes in both engines.
Not yet rerun on the FreeDOS image.

# Code runs anywhere in the program's DOS block — 30 October 2026

The DOS loader's instruction fetch window was the bytes loaded from the file.
Self-unpacking COMs (UPX-style stubs) copy code above the file image and jump
there, so 36 small FreeDOS COMs stopped at Boundary. The window is now the
memory block DOS gives the program at load: PSP through the load-time
allocation end (all conventional memory for a COM; the MZ minimum/maximum
block otherwise). A fetch outside that block, such as a far call into ROM,
still stops as Boundary. The decode cache already re-checks instruction bytes
on reuse, so code written by the guest is decoded fresh.

Verification: 378 Rust tests passed on Linux (one new, two adjusted). The
DOSBox-X CPU (328) and console (8) comparisons still pass in both engines.
Not yet rerun on the FreeDOS image.

# DOS buffered line input (INT21 AH0A) — 30 October 2026

INT21 AH0A reads a line into the DS:DX buffer (size byte, count byte, text).
Keys come from the BIOS ring and are echoed to capture and the active page.
With no key the call waits without retiring and keeps the partial line, so a
line can be typed across several runs. A size byte of 0 returns at once; the
whole buffer must lie inside DS before any key is taken. Up to size-1
characters are kept; a key past that echoes BEL and is dropped. CR is stored
and echoed alone (no LF); the count byte excludes it. LF echoes CR LF (not as
the first key) and is not stored. BS and Left erase the last character on
screen: two columns for a ^X control character, back to the tab stop for TAB.
ESC echoes `\`, CR LF and indent spaces, then restarts the line. F6 stores
^Z. Control characters echo as ^X; ^F and other extended keys are ignored.
The DOS template keys (F1-F5, Right, Ins, Del) stop with "not modeled", and
Ctrl-C at the head of input stops as for the other break-checking reads.
AH0C/AL0A now flushes type-ahead once, then runs AH0A. Behaviour follows the
FreeDOS read_line (chario.c at d6791add); no kernel code is copied.

Verification: 377 Rust tests passed on Linux (five new). The console COM
fixture has eight cases, adding a stuffed-keys line edit (BS) and a full
buffer (BEL). In DOSBox-X 2024.03.01 (headless) all eight console cases pass
and the full output, including the echoed lines, is byte-identical to PIL;
the negative control fails exactly case 001 in both.

# DOS console input and screen-mirrored output — 30 October 2026

DOS stdin/stdout are CON. INT21 AH01 (echo), AH07 and AH08 read the BIOS
keyboard ring; with nothing typed they stop with WaitingForInput without
retiring and resume after Loaded::enqueue_key. An ASCII-0 key returns 00h,
then its scan code on the next read. AH01 echoes every byte it returns,
including that 00h, as the FreeDOS handler does. AH06 DL=FF polls: nothing
ready gives ZF=1/AL=0 without consuming; otherwise ZF=0 and the byte. Other
AH06 DL values output like AH02. AH0B returns AL=FF when input is ready, else
00. AH0C flushes type-ahead (and any owed scan code), then runs AL=01/06/07/08;
other AL values return 00h. A waiting AH0C read flushes once before its first
wait, so the key that ends the wait is kept. AH0C/AL0A is rejected until
buffered input (AH0A) exists.

Ctrl-C (03h) at the head of input stops AH01/02/08/09/0B before any effect,
because DOS break handling (INT23) is not modeled; AH06/AH07 return it as 03h.
Behaviour follows the FreeDOS kernel handlers (inthndlr.c, chario.c at
d6791add); no kernel code is copied.

DOS output from AH02, AH06 and AH09 now also teletypes onto the active text
page, as DOS console output does through the BIOS. The captured output is
unchanged. AH09 TABs expand to the next eight-column stop on the screen. The
screen write set is checked before capture, so a protected cell, bad cursor or
full provider changes neither capture nor RAM.

Verification: 372 Rust tests passed on Linux (six new). The console COM fixture
has seven cases, adding DOS status/read/extended-key/poll/flush; all pass in
PIL and the deliberate failure still fails once.

DOSBox-X comparison (Ubuntu dosbox-x 2024.03.01, headless SDL dummy driver):
all 328 CPU cases and all seven console cases pass in both engines; the
negative control fails exactly case 001 in both. Linux CI now installs
dosbox-x and runs both suites. The comparison found that BIOS AH0E writes to
the active page and ignores BH (as SeaBIOS also does); PIL now does the same.
BEL is not in the COM fixture: DOSBox-X 2024.03 advances the cursor on BEL,
while IBM and SeaBIOS BIOSes do not, and PIL keeps the IBM behaviour.

# BIOS teletype, scrolling, mode and page services — 30 October 2026

INT10 AH0E writes AL at the active page's cursor (BH is ignored, as in SeaBIOS
and DOSBox-X), keeps the cell attribute and handles
BEL (no write), BS (left, no erase, stops at column 0), CR and LF. Column 80
wraps; leaving row 24 scrolls that page up one line and fills the new line with
the attribute under the incoming cursor. TAB is rejected: IBM-derived and
DOSBox-derived BIOSes differ, so no policy is chosen silently.

AH06/AH07 scroll the inclusive CH,CL..DH,DL window on the active page up/down
by AL lines with fill attribute BH. AL=0 or more lines than the window clears
it; corners beyond 24,79 are clamped and an inverted window changes nothing.
AH05 selects active page 0..7 (BDA 0462, start offset 044E). AH0F returns
AH=80, AL=mode and BH=active page. AH00 accepts only mode 3: it resets the
mode-3 BDA fields, all eight cursors and the 0607 cursor shape, and clears all
pages unless AL bit 7 is set. Registers are preserved. Other modes are rejected.

Every service checks its full write set (cells and BDA) before mutating; faults,
bad pages and bad cursors change nothing. No CRTC/VGA ports are modeled. DOS
AH02/AH09 output still goes only to the capture provider, not to the screen.

Verification: 366 Rust tests passed on Linux (seven new service tests). The
console COM fixture grows from four to six cases (teletype controls/wrap/scroll,
active page query and window scroll); all six pass in PIL and the deliberate
readback failure still fails exactly once. Not yet rerun in DOSBox-X.

# DOS BIOS console foundation — 29 October 2026

INT10 AH08 now reads actual character/attribute data from B800:0000; AH09
writes repeated character+attribute and AH0A preserves attributes. The fixed
mode-3 profile has eight 4 KiB pages of 80x25 cells. Writes leave the cursor
unchanged and reject invalid pages, coordinates and visible-page overflow.
The whole write set is checked before mutation. Backing RAM extends to C0000;
the DOS MCB allocator and conventional memory size remain capped at A0000.

INT16 AH00/01/02/05 uses the classic BDA ring (15 usable scan/ASCII words).
Peek leaves the queue unchanged; empty peek sets ZF. An empty blocking read
returns WaitingForInput without retiring the INT. A host adapter can call
Loaded::enqueue_key and resume the returned Loaded state. Queue overflow,
invalid BDA pointers and protected memory have regression coverage. Enhanced
keyboard translation, IRQ1 and keyboard-controller ports are not implemented.

This is a backend foundation, not a visible interactive DOS console. DOS output
capture remains separate. Teletype, scrolling, mode switching, active-page
selection and Python display/input wiring are follow-ups. No ANSI.SYS behavior.

Verification: 359 Rust tests passed on Windows. Four original COM console groups
passed in both DOSBox-X (386/normal) and PIL; a deliberately wrong readback
produced exactly one failure in each. Existing 328 CPU COM cases also passed in
both engines. This does not establish physical 386 or full BIOS compatibility.

# DOS COM/EXE import and instruction-gap probe

This probe loads headerless COM or DOS MZ/ZM EXE files and runs the implemented
PIL instruction subset. It reports the first blocker, source file offset, raw
bytes, register context and reached instruction frequencies. It can rank the
first missing CPU encodings across a collection of programs.

## First run: original hello-world programs

After `sh tools/build.sh` on Termux/Linux, create the supplied examples:

```sh
./target/debug/pil --write-dos-hello com target/dos-hello.com
./target/debug/pil --write-dos-hello exe target/dos-hello.exe
./target/debug/pil --dos-probe target/dos-hello.com target/dos-hello.exe
```

PowerShell, after `.\tools\build.ps1`:

```powershell
.\target\debug\pil.exe --write-dos-hello com target/dos-hello.com
.\target\debug\pil.exe --write-dos-hello exe target/dos-hello.exe
.\target\debug\pil.exe --dos-probe target/dos-hello.com target/dos-hello.exe
```

Expected: both produce `OUTPUT (escaped bytes): Hello, DOS!\r\n`, stop with
`Exit(0)`, and report `SUMMARY imported=2/2 exited=2`. Each executes three PIL
instructions and two bridged DOS calls. These are original tiny DOS executables,
not the existing monitor-port hello.bin. Creation refuses to overwrite files.
The EXE deliberately addresses its string relative to the initial PSP DS so it
can run before segment-register move instructions are implemented.

## Build profile and executable path

The default build helpers create debug binaries: `sh tools/build.sh` builds
`target/debug/pil`; `.\tools\build.ps1` builds
`target/debug/pil.exe`. They do not refresh an existing release binary.
Run the path reported by the build helper, as in the examples above.

For an optimized build, use `sh tools/build.sh --release` or
`.\tools\build.ps1 -Release`, then replace `debug` with `release` in the
executable paths. If you configured `CARGO_TARGET_DIR`, use that directory
instead of `target`.

If a DOS option prints the old `AX=0x8000; ... typed tokens` demonstration,
you ran an older executable that does not recognize those options. Select the
newly built debug binary or explicitly rebuild release; no files are generated
by that old fallback demonstration.

## Import your programs

```sh
./target/debug/pil --dos-probe --steps 10000 program.com program.exe
```

Use quoted paths for filenames containing spaces. The optional step budget must
precede the filenames and applies to each program. Default:10,000; maximum:
1,000,000. Up to256 input files are accepted per report, each at most1MiB.
The diagnostic command returns a report even for import/execution blockers;
inspect the `STOP` and `SUMMARY` lines rather than treating its host exit status
as a DOS program success code.

At the debugger prompt, `dosprobe file.com 10000` runs the same isolated probe
without replacing or modifying the current monitor program. For paths with
spaces use the CLI; the REPL uses whitespace-separated arguments. The existing
`load` command still loads raw monitor images; use the DOS probe for DOS files.

## Interpreting the report

| Result | Meaning |
|---|---|
| `Exit(code)` | Reached a supported DOS termination call |
| `MissingEncoding` | Decoder stopped on an unsupported instruction/form/prefix |
| `Service` | Unsupported DOS/BIOS call or a bounded DOS-string service failure |
| `Fault` | Shared execution or fetch/segment/RAM check failed |
| `Boundary` | Instruction fetch left the program's DOS memory block (PSP to load-time allocation end) or truncated an instruction |
| `Budget` | Per-program execution limit reached |
| `Monitor` | Reached HLT or monitor-port I/O, which are not DOS termination/services |
| `IMPORT ERROR` | File/header/relocation/allocation checks rejected the input |

A typical relocated EXE startup can now load AX and execute `8E D8`
(`MOV DS,AX`) in real mode. The regression fixture then reaches an intentionally
unsupported `62` opcode at file offset37. That is a CPU gap; an
unimplemented `INT21/AH30` DOS-version query is a separate service gap.
Raw bytes are shown for context and are not claimed to be a decoded length.
`file_offset` is the position in the original COM file or MZ load module plus
header; it is absent for PSP or other addresses outside the module. Stop-state
registers and CS:IP show where execution reached, including after prior writes.

The missing-encoding histogram counts programs blocked by each first opcode,
not an estimate of every missing instruction in those programs. The reached
instruction mix counts fully retired instructions and lists bridged DOS calls
separately. The per-program step budget includes REP element steps (and bridged
services), so large repeats cannot bypass it. PIL execution counts include each
element; completed counts include the REP instruction only after full retirement.
Separate REP element mix entries count committed elements; zero-count REP
retires without a data access. A later fault preserves earlier element progress. Unknown encodings are never skipped; code/data bytes are not scanned
as if all were instructions. Fix a common blocker and rerun the same corpus to
reveal the next one. Dynamic paths, inputs and loops affect the frequencies.

## Real-mode CS data access

Ordinary COM/MZ CS caches permit data reads and writes, including CS overrides
used by startup and unpacking code. Segment limits and RAM write protection
still apply. Modified instruction bytes invalidate the interpreter decode cache
on reuse. Fetch is allowed anywhere in the program's block (PSP through the
load-time allocation end), so unpacked or relocated code runs. Explicit protected Code caches keep
their read/write restrictions. Fixed compiled images retain changed-code guards.

## Loader and service contract

- The probe places a minimal PSP at segment1000h in zero-filled conventional
  memory belowA0000h. It provides INT20 at PSP:0, the allocated-end segment,
  an empty command tail and no environment block or real DOS memory arena.
- COM starts at CS:IP=1000:0100 with CS/DS/ES/SS=PSP and SP=FFFEh. A zero
  stack word is installed there, including overwriting the last two bytes of
  a maximum-size65,280-byte image as part of that initial stack convention.
- MZ imports validate page counts, header/module sizes, relocation tables,
  relocation targets, initial entry and stack, min/max allocations and overlays.
  Relocations add the load segment1010h with16-bit wrapping, in table order.
  CS/SS are relocated with16-bit wrapping; DS/ES point to the PSP.
  Wrapped relative segments such as FFF0:0100 are accepted when the resulting
  physical entry lies inside the load module. Zero min=max high-load files,
  overlays and recognized NE/LE/LX/PE/W3/W4 containers are rejected. Trailing
  bytes beyond the declared DOS image are reported and not loaded. Checksum
  validation is not required; the MZ checksum field is ignored.
- Initial general registers are zero except SP, flags are0202h, and FS/GS use
  the PSP cache as a probe convention. These are not universal DOS entry-state
  promises. SS:SP is conservatively required to fit the assigned memory block.
- Execution uses the concrete PIL token interpreter and its existing decoder
  and access checks. DOS probing is not currently a generated-Rust runner.
  COM fetches are bounded to PSP plus imported bytes; MZ fetches to the loaded
  initial-CS code window. A20 wrap and executable code outside those windows
  are not modeled. Stack/segment/control instructions remain limited to what
  the current instruction profile implements.
- Bridged services include:
  - **INT 10h:** AH=03h (read cursor position and size), AH=08h (read character and attribute).
  - **INT 11h:** BIOS equipment list word determination.
  - **INT 12h:** Conventional memory size query (640 KB).
  - **INT 1Ah:** AH=00h (read system-clock timer ticks).
  - **INT 20h:** CS=PSP exit.
  - **INT 21h:** AH=02h/AH=09h (output), AH=25h/AH=35h (set/get vector), AH=30h (version query), AH=37h/AL=00h (switch query), AH=3Dh (open file failure stub with CF=1, AX=0x0002), AH=3Eh (close file stub with CF=0), AH=48h/49h/4Ah (bounded memory), AH=4Ch (exit).
  Output is capped at 16KiB and a service checks the whole string before
  publishing output. AH09 returns AL=24h and preserves other modeled state.
- These calls are an explicitly labeled host shim, not CPU INT/IRET semantics:
  no interrupt frame, IVT dispatch, BIOS, files, keyboard, environment, memory
  allocation strategies, Ctrl-Break handling or full DOS process lifetime exists.
  Captured bytes are displayed escaped; no CP437 screen rendering is implied.

## Sources and verification

Original hello fixtures and tests are authored for this project; no external
DOS programs, emulator implementation or interrupt-list text is bundled.
Header/entry/relocation contracts were checked against Microsoft's published
[MS-DOS EXEC source](https://github.com/microsoft/MS-DOS/blob/main/v4.0/src/DOS/EXEC.ASM)
and [header definitions](https://github.com/microsoft/MS-DOS/blob/main/v2.0/source/DOSSYM.ASM),
plus the archived [MS-DOS3.3 Programmer's Reference](https://www.pcjs.org/documents/books/mspl13/msdos/dosref33/).
The small output/termination bridge follows the documented DOS API; AH09's
AL=24h behavior is recorded in [Ralf Brown's Interrupt List](https://delorie.com/djgpp/doc/rbinter/id/73/25.html).

Development checks include malformed and relocated MZ images, stack/allocation
bounds, hello execution, output limits, first-blocker reports, bounded loops,
and debugger isolation. `python tools/verify_dos_probe.py` additionally authors
its relocation fixture independently and checks actual CLI output. Runtime export
and native-package smoke checks run both COM and MZ hello files.

Verified on Windows/Linux CI40460972486 at source11dd1cae:200 tests per host,
independent DOS CLI fixtures and both COM/MZ hello examples in standalone source
and unpacked native packages passed. The existing49 backend/native CPU regression
suite also passed; it does not compare DOS-shim execution with generated Rust.

## Disk images and ZIP packages (0.20 alpha)

`--dos-probe` and REPL `dosprobe` also accept FAT16 `.img`/`.ima` and `.zip`
inputs. `--dos-list PATH` lists candidates; `--dos-extract PATH NEW_DIRECTORY`
extracts regular files and one package ZIP level without changing the source.
See [container guide](CONTAINERS.md) for FreeDOS commands, formats and bounds.
Container executables are deduplicated by full byte equality before probing.

## Bounded conventional-memory services (0.27 implementation)

INT21/AH48 allocates first-fit BX paragraphs and returns segment AX;AH49 frees
ES;AH4A resizes ES to BX paragraphs. CF is defined clear on success or set with
AX7 (corrupted MCB),8 (insufficient memory) or9 (invalid/unowned block). Insufficient-memory AH48
and insufficient-growth AH4A return the largest available size in BX. DOS2..6
resize failure still grows to the maximum contiguous size. Upper register
halves and other flags/markers are preserved. Free does not zero bytes;
adjacent free blocks coalesce during allocate/resize,not free. Zero paragraphs
are represented by an owned zero-data MCB. First-fit only; no UMB/strategy,
EXEC/TSR,environment allocator,shared processes or device-owned blocks.

The arena spans MCB0FFFh through conventional-memory end A000h. Each MCB uses
one paragraph with type M/Z,owner PSP (1000h) or0 and data size in paragraphs.
The loader writes a real initial chain and maintains trusted host metadata.
Before services,mirrored type/owner/size must match; direct guest chain edits
are rejected with DOS7 rather than silently overwritten. Names/reserved bytes
are not interpreted. A4096-block safety limit stops the service diagnostically.

Physical probe RAM is now always640KiB,independent of MZ initial allocation.
PSP:0002 retains the original load-time end,as in the FreeDOS handler;
allocation_end tracks the current owned PSP block separately. Successful4A
returns the block segment in AX. Allocation is DOS bookkeeping,not hardware memory protection:
freed/unallocated RAM remains addressable in real mode. The instruction
fetch window is the load-time program block; AH48 blocks above it are not
executable. All MCB writes are prechecked before committing metadata or
register results; readonly header-write failures stop the bridge without partial state.

Reference audit: [FreeDOS INT21 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c), especially4A PSP-size retention and AX=ES; [RBIL48](https://fd.lod.bz/rbil/interrup/dos_kernel/2148.html), [49](https://fd.lod.bz/rbil/interrup/dos_kernel/2149.html), [4A](https://fd.lod.bz/rbil/interrup/dos_kernel/214a.html) for the bounded DOS2..6 service profile.

## Real-mode far returns (0.29)

CB and CA imm16 now restore CS:IP and advance SP using shared PIL. Operand16/32
and SS size are independent;the selector uses the low16 bits of its stack slot.
A bad complete frame,new IP beyondFFFFh or RAM access fault commits no return
effects. Flags are preserved. Protected/V86 far returns and far calls/jumps are
not implemented. Fetch bounds now refer to the initial physical window across
CS changes;this permits aliases of imported code without permitting execution
of unpacked/copied code outside the original bounded window.

## DOS version compatibility profile (0.30)

The bounded host shim now accepts INT21/AH30. Its default declared compatibility
profile is6.22 (AX=1606h),initialized in the original PSP at offset0040h.
Guest edits to that word affect subsequent queries. AL=1 on entry selects
non-ROM version flags (BH=0);all other AL values select OEM FF. BL and CX are
zero;register high halves,other registers,EFLAGS and undefined markers are
preserved. A failed PSP read changes no return registers or output. This is
metadata for the shim,not the version of a booted kernel or a claim of complete
DOS6.22 functionality. Other unsupported DOS/BIOS services remain blockers.

The implementation was checked against the
[FreeDOS AH30 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c)
for input AL,PSP version and serial fields. The implementation is original;
no kernel code is copied and the shim does not claim to be FreeDOS.

### Measured memory-PUSH progress (0.31)

The actual FD14LITE.img was probed locally on Windows with 100000 steps.
All sixteen FreeCom variants advanced from `FF 74 14` (memory PUSH /6) to
`FF 1E 1F 02` (far CALL /3), twelve bytes later. Imports remain 151/156
and exits remain six. The grouped `opcode ff` total is therefore unchanged;
compare per-program stop details, bytes and CS:IP to measure this progress.
The disk image and third-party programs are not bundled in the repository.

Measured image SHA256: `f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36`.

### Measured far-CALL progress (0.32)

With the same actual image and 100000-step budget, all sixteen FreeCom variants
pass FF /3 and stop at DOS INT 21/AH=25. GREP passes 9A and stops at memory XCHG
(`87 0E 7A 11`). Missing-encoding program counts are now two `0F BB`, one
`0F 01`, one `60`, and one `87` (five total, previously 21). Imports stay
151/156 and exits stay six. This is first-blocker progress, not completion of
these programs or a booted DOS kernel.


## Interrupt-vector services (0.33)

AH25 stores DS:DX at physical AL*4 as little-endian offset then selector.
AH35 reads that entry into ES:BX and rebuilds the ordinary real-mode ES cache.
Upper EBX and all flags are preserved. The four-byte RAM access is checked
before mutation; failed calls do not retire. Handler pointers are not executed
or dereferenced. Guest IVT writes are visible to AH35. Vectors initially contain
zero; no DOS/BIOS handlers are supplied and CPU interrupt dispatch remains
unimplemented. The existing host shim still intercepts supported INT calls.

Contracts follow Microsoft MS-DOS Encyclopedia INT21 Set Interrupt Vector
(AH25) and Get Interrupt Vector (AH35), and Ralf Brown's Interrupt List.
No third-party implementation copied.

## DOS output, switch query and LES (0.34)

INT21/AH02 reads DL, appends the byte to captured output and returns it in AL.
A tab expands to spaces up to the next eight-column boundary and returns AL=20h.
The column is derived from prior captured bytes: CR resets it, LF and BEL
preserve it, BS decrements toward zero, and tabs advance to the next tab stop.
Other bytes are captured unchanged and advance the column. Existing AH09 capture
remains unchanged. The whole AH02 expansion must fit the16KiB cap before output
or AL changes. Other registers and flags are preserved. Keyboard/break handling,
redirection and a screen/BIOS device are not implemented.

INT21/AX3700 returns AL=0 and DL='/' for the declared DOS6.22 compatibility
profile, preserving upper register bits, other registers and flags. AH37/AL01
and other subfunctions remain explicit unsupported-service stops.

LES C4 accepts real-mode memory16:16/16:32 pointers with independent16/32-bit
address sizes and ordinary segment overrides. Both fields use incoming address
registers and source caches, including an ES override; the full pointer must fit
the source segment and RAM. GPR and ES selector/cache commit together. Flags
are preserved. No protected/V86 load, descriptor lookup or other full-pointer
load instruction is added. Pointer segment-end wrap is rejected by this profile.

Report headers print the package version and selected per-program step budget,
including the default budget when --steps is omitted. Keep the supplied v0.33
100000-step report as historical user evidence until the new corpus is rerun.

## Unsigned division and BIOS / DOS service shims (0.35)

- Unsigned division `DIV` (`F6 /6`, `F7 /6`) is supported at 8, 16, and 32 bits (with 66h prefix) for both register and memory addressing modes. AX, DX:AX, and EDX:EAX dividends produce quotient and remainder in AL/AH, AX/DX, and EAX/EDX. Zero divisor and quotient overflow are checked via PIL `require`. Arithmetic flags CF, OF, SF, ZF, AF, PF are marked undefined.
- Foundational BIOS and DOS services added to the shim profile:
  - **INT 10h (AH=08h):** Read character and attribute at cursor; returns `AH=07h` (standard light gray attribute) and `AL=20h` (blank character).
  - **INT 10h (AH=03h):** Read cursor position and size; returns `CX=0607h` (standard CGA scanlines) and `DX=0000h` (row 0, column 0).
  - **INT 11h:** BIOS equipment list word; returns `AX=0025h` (pointing device installed, 80x25 color display, 1 floppy drive).
  - **INT 12h:** Conventional memory size query; returns `AX=0280h` (640 KB conventional RAM).
  - **INT 1Ah (AH=00h):** Read system-clock timer ticks; returns `AL=0` (midnight flag clear), `CX=0`, `DX=0` (tick count 0).
  - **INT 21h (AH=3Dh):** Open existing file; returns bounded failure fallback with `CF=1` and `AX=0x0002` (file not found).
  - **INT 21h (AH=3Eh):** Close file; returns success stub with `CF=0`.
- Probe report progress: clean exits increased from 11 to 14, FreeCOM shells advance through initialization loops without crashing on missing equipment/video queries, and memory utilities (such as MEMSTAT) calculate and display formatted decimal output using `DIV`.

