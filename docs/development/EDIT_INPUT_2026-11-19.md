# EDIT input investigation — 19 November 2026

The user reports delayed mouse/menu response in FreeDOS EDIT after
`START4.BAT`, including through the normal PILdos shortcut. Creating a new
document and clicking its editing area can make the mouse and Alt menus
appear unresponsive while ordinary typing continues. The text appears over
the old caret position; the caret follows later, sometimes after the next
letter has been typed. COMMAND.COM's prompt handles rapid typing normally.

## Confirmed keyboard defect

Native input used the guest IRQ1 path when INT 09h was hooked, but otherwise
put translated characters directly into the BIOS ring. The latter path
discarded modifier events, leaving INT 16h/AH=02h's Shift/Ctrl/Alt state stale.
Both paths now use the same BIOS modifier update logic. The guest IRQ1 path
still updates the flags when its handler chains to BIOS, rather than ahead
of interrupt delivery.

An original nine-byte BIOS-polling fixture reproduced the missing Alt bit
before the change. It now verifies held Alt, the Alt+F BIOS word, releases,
Shift and extended Ctrl, preserved toggle bits and focus loss. All 34 host
console tests and 11 keyboard tests passed. This is not a claim that the
complete interactive EDIT failure is fixed.

## Remaining latency investigation

A temporary, uncommitted session-protocol probe ran the user's unchanged
programs with the existing release engine (SHA256
`03f71afba38391519178baf46668bd3d147860b0051033e90e2211eec8488cc1`).
It opened EDIT from FreeCOM, and separately after START4, chose File/New,
and entered three letters. In both runs it observed the new character
before the BIOS cursor advanced. Approximately 58,000 guest instructions
separated those observations. The measured intervals were 136–154 ms under
FreeCOM and 135–150 ms under 4DOS. These are single-run, headless observations
sampled every 20,000 execution steps; timing includes protocol and screen
snapshot overhead and is not a native-window end-to-end latency measurement.

The session probe could still open File by mouse and Alt+F after document
creation when it allowed guest execution between input transitions. A complete
Windows reproduction and the cause of the reported extra mouse delay remain
unconfirmed. No driver routine has been identified as the cause, and no
application-specific timing or cursor workaround is added.
