### Summary
Comprehensive update combining bug fixes, resource leak cleanups across frontends/engine, and performance optimizations for the DOS execution loop, PIL compiler, and Tk terminal UI.

### Part 1: Bug Fixes & Resource Cleanup
* **`dos_terminal.py`**:
  * Support dynamic text-mode screen geometries (`columns` and row count changes) instead of hardcoding 80×25 newline bounds.
  * Use an incremental UTF-8 decoder (`codecs.getincrementaldecoder`) in `ConsolePort._read_posix` to prevent dropping split multi-byte characters.
  * Properly close client/server sockets in `TcpPort` and subprocess pipes in `Session.close()`.
  * Validate signed `--utc-offset` values against the full `-12:00..+14:00` range.
* **`pil_debugger.py`**:
  * Pass `CREATE_NO_WINDOW` to helper `subprocess.run` calls (`--dos-read`, `--dos-probe`, `--dos-list`) to prevent console window flashes on Windows.
  * Drain completed background jobs on app shutdown (`App.close`) so temporary extracted `.bin` files are cleaned up.
  * Compute hex byte column offsets dynamically from `:` in memory dump highlighters.
* **Rust Engine (`pil` / `dine`)**:
  * **`main.rs`**: Preserve non-CF lazy flag state in `LazyFlags::flush_cf_only` by clearing `write_cf` instead of resetting the entire enum to `LazyFlags::None`.
  * **`guest.rs`**: Set `disp_bits = 32` for indirect `CALL` (`FF /2`) instructions.
  * **`debugger.rs`**: Clear the decode cache and reset `pil_cursor` on `poke`, and reset `pil_cursor` on `set` commands.
  * **`dos_files.rs`**: Add `asciiz_in` to read `ES:DI` paths without mutating `DS`, use `saturating_sub` for cluster deltas, and keep `Arc<Vec<u8>>` file buffers synchronized.
  * **Diagnostics**: Fix missing spaces in error and report strings across modules.

### Part 2: Performance Optimizations
* **`dos.rs`**:
  * Persist `DecodeCache` inside `Loaded` so `--dos-session` retains decoded PIL programs across execution slices instead of starting cold every `run` slice.
  * Use `execute_machine_lazy` across the `run_with_output` loop and flush `LazyFlags` only on `INT` traps or loop exit.
  * Check `0xCD` (`INT`) via stack-allocated `fetch_signature` instead of allocating a `Vec<u8>` via `bytes_at` on every instruction.
  * Look up instruction mix counters by `&str` (`get_mut`) before allocating a `String` key, avoiding a heap allocation on every retired instruction.
* **`main.rs`**:
  * Use borrowed `&str` keys in `compile()`'s symbol table (`HashMap<&str, usize>`) to avoid `.to_owned()` allocations per defined PIL value.
  * Trim `execute_state`'s stack-allocated `value_stack` fast path from 64 to 24 slots.
* **`dos_terminal.py`**:
  * Update only the `cursor` tag when the screen buffer is unchanged.
  * Diff `screen_rows` against the previous frame and only delete/re-insert modified rows when geometry is unchanged.