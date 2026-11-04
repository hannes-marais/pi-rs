2026-11-04

# QuickJS C11 atomics across Windows compilers

Owner Codex; EMgine VPM vm-a0ks. ACC /home/oumy/work/emgine/config/man/anvil.md. Architecture principles 3, 6, 13, 21, 24, 27, 30 and 35 govern compiler lowering, target identity and evidence. ToQ: compile the real QuickJS source for Windows with both MSVC and the enrolled clang-cl cross toolchain; ToT: ask the existing cc compiler abstraction whether the optional atomics flag is supported.

The build script unconditionally inserts MSVC's `/experimental:c11atomics` in CFLAGS for every MSVC-ABI target. clang-cl treats this unsupported flag as an input filename and fails before compiling QuickJS. Keep C11 and WIN32_LEAN_AND_MEAN, add the atomics switch only through cc's supported-flag probe, and preserve native non-Windows behavior. Validate Linux and the pinned Linux-to-Windows xwin compile of rquickjs-sys without a compiler wrapper. A full pi-jsrt Windows build still requires its host APIs and collation to be ported. Risk: compiler probing must correctly reject unsupported clang-cl while keeping the flag on genuine MSVC.

Updated: 2026-11-04. `cargo xwin check --offline --locked -p rquickjs-sys --target x86_64-pc-windows-msvc --cross-compiler clang-cl --xwin-arch x86_64 --xwin-variant desktop` passed with the pinned Rust 1.98 and clang-cl toolchain and a private x86-64 SDK cache, without the diagnostic wrapper. Linux `cargo check --offline --locked -p rquickjs-sys` passed. The full pi-jsrt Windows check now reaches Rust and still reports 211 platform errors. OptiPlex1 has no `cl.exe` on PATH, so native MSVC flag probing remains unverified.
