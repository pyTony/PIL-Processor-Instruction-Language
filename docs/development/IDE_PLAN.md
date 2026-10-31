# IDE Architecture and Implementation Plan — lispy machine layout

28 October 2026. This plan defines the Integrated Development Environment (IDE)
architecture for PIL and guest x86 execution, structured directly around the
tripartite machine layout established by the user's `lispy3` project (`lispy_machine.pyw`),
supplemented by the CPU debugging patterns from `z80-cpm` (`debug_gui.pyw`).

## 1. Architectural inspiration: lispy machine

In `lispy3`, `lispy_machine.pyw` organizes the development environment into three
primary interacting surfaces:
1. **Editor**: Source text editing, file lifecycle (new, open, save), and evaluation/build dispatch.
2. **Listener**: Interactive REPL for direct command entry, evaluated expression returns, and batched diagnostic logging.
3. **Inspector**: Structured view of runtime internal state, bindings, environments, and diagnostic details.

Crucially, `lispy3` enforces strict thread separation:
- The UI runs on the Tk main thread and never blocks.
- Execution runs on an independent worker/process boundary.
- UI updates and listener logs are queued and batched to prevent interface freezing.
- Program output and development evaluation channels are kept strictly distinct.

## 2. PIL IDE layout and components

The PIL IDE adapts this tripartite structure for low-level systems and CPU development:

```
+-----------------------------------------------------------------------------------+
| Top Toolbar: [Open binary...] [Assemble/Load] [Step F7] [Run F5] [Pause F6] [Reset] [Demo v] |
+-------------------------------------------------+---------------------------------+
| PRIMARY WORKSPACE (Left: 3/5 width)             | INSPECTOR PANE (Right: 2/5 width)|
| +---------------------------------------------+ | +-------------------------------+ |
| | [Tabs: Disassembly / Source Editor]         | | | Registers (Treeview with       | |
| | - Disassembly with active * IP indicator    | | |   change highlighting)        | |
| | - Breakpoint indicators (red)               | | +-------------------------------+ |
| | - Dual-click to toggle breakpoint           | | | Memory & Stack Viewer       | |
| +---------------------------------------------+ | | - SS:ESP relative stack frame | |
| | Breakpoint bar: [0x100] [Toggle F9]         | | | - Configurable RAM window     | |
| +---------------------------------------------+ | +-------------------------------+ |
| | Guest Terminal / Output                     | | | Machine & Segment Inspector   | |
| | - Raw guest serial & DOS console stream     | | | - Flags & Undefined metadata  | |
| | - Escaped binary / text view                | | | - Segment caches (CS,DS,SS,ES)| |
| | - [Guest RX Entry] [Inject]                 | | | - Instruction cache stats     | |
| +---------------------------------------------+ | +-------------------------------+ |
+-------------------------------------------------+---------------------------------+
| LISTENER & COMMAND REPL (Bottom: full width)                                      |
| +-------------------------------------------------------------------------------+ |
| | Log / Listener Output (batched engine messages, command echoes, check results)| |
| +-------------------------------------------------------------------------------+ |
| | Command Entry: pil> [command with Up/Down history] [Send]                     | |
+-----------------------------------------------------------------------------------+
| Status Bar: Current execution status, stop reasons, and error notices            |
+-----------------------------------------------------------------------------------+
```

### 2.1 The Editor pane
- **Source Editor & Disassembly Tabs**:
  - *Disassembly Tab*: authoritative listing provided by the running engine's decoder, showing exact address, opcode bytes, instruction text, and current instruction marker (`*`).
  - *Source Editor Tab*: allows authoring small assembly scripts or PIL instruction snippets directly in the IDE without leaving the tool.
- **Assemble & Load Action**: Invokes the fixture builder / assembler to produce a validated raw binary or PIL snippet and loads it directly into the engine client (`load <path>`).
- **Breakpoint Interaction**: Double-clicking an address in the listing or editor toggles a breakpoint through the engine protocol.

### 2.2 The Listener pane
- **REPL / Command Line**: Direct access to the engine CLI protocol (`run <n>`, `step`, `break <addr>`, `delete <addr>`, `regs`, `flags`, `view <addr>`, `check`, `reset`).
- **Command History**: Up and Down arrow key navigation cycling through previously executed commands (as in `lispy_machine`).
- **Output Batching & Rate-Limiting**: Engine replies and log events are appended in batches with bounded line retention (preventing memory exhaustion during long runs).
- **Channel Distinction**: Debugger command logs and system diagnostics are displayed in the Listener; guest output is routed exclusively to the Guest Terminal.

### 2.3 The Inspector pane
- **Registers**: EAX, ECX, EDX, EBX, ESP, EBP, ESI, EDI, EIP, EFLAGS formatted in hexadecimal, with tags highlighting registers that changed in the last execution quantum.
- **Flags & Undefined Metadata**: Separate breakdowns of concrete EFLAGS (CF, PF, AF, ZF, SF, TF, IF, DF, OF) and PIL's deterministic diagnostic undefined-flag mask (`undefined_flags`).
- **Segment Descriptors & Hidden Cache**: Real-mode and protected-mode cache views for CS, DS, SS, ES, FS, GS (selector, base, limit, access rights).
- **Stack & Memory Views**:
  - Active stack frame inspection relative to SS:SP/ESP.
  - RAM inspection window with configurable hexadecimal address input.
- **Decode Cache Telemetry**: Live hit, miss, and invalidation counters from the execution engine.

### 2.4 Guest Terminal
- **Stream Separation**: Receives guest character emissions (mock serial port 0xE9, DOS INT 21h AH=02h/09h).
- **Binary/Control Escaping**: Preserves exact byte streams while safely rendering non-printable and binary output.
- **RX Injection**: Input bar allowing interactive injection of ASCII or hex bytes into the guest serial RX queue.
- **Future Screen Model**: Evolution path to render the 80x25 character/attribute video matrix for INT 10h text mode.

## 3. Concurrency and Process Model

1. **Host-Independent Engine Client (`EngineClient`)**:
   - The UI communicates with `pil --debug-service` over stdio using line-delimited JSON.
   - A dedicated worker thread manages the subprocess, ensuring the Tk main thread never blocks on execution or I/O.
2. **Bounded Communication Queues**:
   - Outgoing commands queue: `maxsize=1` (at most one command in flight at a time).
   - Incoming events queue: bounded to 8 items to prevent runaway memory if the UI lags behind engine emissions.
3. **Cooperative Run Quanta**:
   - Execution is scheduled in bounded quanta (e.g., 256 steps).
   - Pause does not kill threads; it cooperatively ceases scheduling the next quantum at the next instruction retirement.

## 4. Implementation Stages

- **Stage 1 (Current Baseline)**: `tools/pil_debugger.py` Tk frontend with disassembly, registers treeview, memory, stack, inspector, guest console, and listener.
- **Stage 2 (Lispy Machine Alignment)**:
  - Add command history (Up/Down arrows) to the Listener entry.
  - Add Source Editor tab alongside disassembly listing with open/save capability.
  - Add Listener output controls (Clear, filter).
- **Stage 3 (Build & Assemble Integration)**:
  - Wire build/assemble actions from the Source Editor into the running engine session.
  - Implement syntax highlighting for assembly and PIL files.
- **Stage 4 (Screen Buffer & DOS Machine)**:
  - Add emulated character/attribute text-screen viewer alongside the serial console.
