# Android ARM64: inspect generated native PIL Rust before compiling

This experiment preserves a **source-first workflow**. It uses the existing `pil --emit-suite-rust` code generator to emit a host-native Rust source file for the project's fixed x86 verification images, then compiles that file directly with Termux's `rustc`. Generated instruction bodies come from validated PIL, rather than interpreting PIL tokens. It does **not** compile arbitrary DOS COM/EXE programs, and it does **not** produce an ESP32 application.

## On Termux

Use your proven memory-constrained script to build the source generator if needed:

```sh
sh tools/build_termux.sh --compact
```

Generate *only the Rust source* (this does not run rustc):

```sh
sh tools/native_android_proof.sh --emit
ls -lh target/native-android-proof/generated.rs
sed -n '1,100p' target/native-android-proof/generated.rs
```

Browse the generated Rust with an editor or `less` before going further. The source remains in `target/native-android-proof/generated.rs` and is not overwritten by `--build`.

When satisfied, compile a native executable with a low-memory, single-rustc invocation:

```sh
sh tools/native_android_proof.sh --build
ls -lh target/native-android-proof/generated
```

Run the generated executable's internal verification:

```sh
sh tools/native_android_proof.sh --run
```

The generated program writes `target/native-android-proof/generated.trace`. Unlike `pil --verify-backends`, the script does **not** compare the trace with a reference interpreter trace. A successful run proves that its built-in cases execute and assertions pass; it does not establish full equivalence. Use the existing backend-verification pipeline for the interpreter-versus-native comparison when resources permit.

## Scope and follow-up

- The emitted cases include arithmetic, memory, control flow, and selected protected-mode operations; they are fixed proof images, not arbitrary DOS programs.
- `rustc` builds for the **host**. In Termux on an ARM64 phone this yields an Android-compatible AArch64 executable using the host's normal runtime.
- The script deliberately does not invoke an unrestricted Cargo release build; source emission, compilation, and execution are separate. Even a single rustc invocation may exceed memory on some devices. Stop on resource pressure rather than retrying with increased parallelism.
- The earlier per-program DOS native compiler was removed from current `dev`. `tools/build_dine.py` produces guest-byte launchers backed by a common interpreted Dine runtime. Neither is represented as a fully native DOS program in this experiment.
- Next engineering step: add a program-independent offline compiler API which emits inspectable Rust for supported whole DOS programs, records compiled coverage and unsupported dynamic edges, and links against a shared program-independent DOS runtime. Start with HELLO and FACTOR, and require no interpreter fallback before calling an artifact fully compiled.
