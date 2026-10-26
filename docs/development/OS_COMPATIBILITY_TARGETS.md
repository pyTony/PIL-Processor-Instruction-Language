# Later operating-system compatibility targets

Updated 27 October 2026 following the user's comparison table. These are
proposed acceptance targets, not current boot support or guaranteed TTGO fits.
Pin exact edition, release/build, drivers and installation media before testing.

| Milestone | CPU/RAM reference | Project interpretation |
|---|---|---|
| Plain Windows 3.1x | Separate product from Windows for Workgroups; do not assign the latter's minimum automatically | First Windows-family goal after DOS/BIOS/device and protected-mode work; choose exact edition and initial execution mode |
| Windows for Workgroups 3.11 | Microsoft KB126746:386SX+,3MB RAM;2MB without networking,4MB recommended | Enhanced-mode goal with networking initially off; add network adapter/stack later |
| Windows 95 | Microsoft KB138349:386DX+,4MB minimum,8MB recommended | Pin retail/RTM as initial candidate; later OEM/OSR builds need separate checks |
| Windows NT Workstation 3.51,x86 | User-supplied12MB target,corroborated by an indexed Digital announcement listing386/25+ and12MB; Server16MB | Optional later protected-mode/paging/interrupt/device milestone; verify exact edition media/HCL before claiming official coverage |
| Successor boundary:Windows 98 | Microsoft KB182751:486DX66MHz+,16MB | Separate486-class target,not evidence of a386 requirement |
| Successor boundary:Windows NT4.0 | Microsoft KB126690 setup guide lists486/25+ | Separate486-class target,not an implicit extension of the386 profile |

The3MB row in the supplied table belongs to **Windows for Workgroups3.11**,
not a generic minimum for every product called Windows3.11. Networking changes
the WfW minimum. NT Workstation and Server also need distinct RAM rows.
Advertised minima are neither recommended emulator configurations nor evidence
that a particular board can supply the guest memory efficiently.

Treat each OS as a versioned compatibility milestone: installation or known
boot image, desktop/command startup, file I/O,input/display,shutdown/reboot and
a small application test. Log CPU model,RAM,BIOS/device versions,guest build,
disk hash and successful scenarios. RAM alone or instruction-family coverage
does not prove OS compatibility. Host-first validation precedes embedded memory
budgeting and SD/PSRAM/storage design.

Suggested progression: tiny reset/real-mode diagnostic -> external CPU/ROM
checks -> DOS-style boot/file path -> selected plain Windows3.1x mode ->
386 enhanced/WfW3.11 -> optional Windows95 -> optional NT3.51. Dependencies
can change the latter ordering; neither is a near-term delivery promise.

Requirements references consulted27 October2026:
- Microsoft KB126746,archived Microsoft WfW version history:
  https://ftp.zx.net.nz/pub/archive/ftp.microsoft.com/MISC/KB/en-us/126/746.HTM
- Microsoft KB138349,archived Windows95 installation requirements:
  https://ftp.zx.net.nz/pub/archive/ftp.microsoft.com/MISC/KB/en-us/138/349.HTM
- Indexed Digital NT3.51 product announcement:
  https://groups.google.com/g/biz.digital.articles/c/C2PUhJoa1-c
  Search retrieval supplied its technical-data excerpt; direct opening was rate
  limited. The obsolete reviewer-guide URL redirected to an unrelated modern
  landing page and was not used as evidence.
- Microsoft KB126690,archived NT4 setup guide:
  https://jeffpar.github.io/kbarchive/kb/126/Q126690/
- Microsoft KB182751,archived Windows98 requirements:
  https://helparchive.huntertur.net/document/106160

No OS binaries,manual text or media are bundled. This is original planning
prose with requirement facts attributed to the above sources.
