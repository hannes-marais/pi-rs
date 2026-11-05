2026-11-05

# Windows POSIX API boundaries in Pi

Owner: Codex; EMgine VPM `vm-wgv2` (Windows Anvil build) and `vm-yg8u` (fdevbox storage retention). Outcome: lower Pi's DNS, IPC and process APIs through native Windows implementations or explicit, truthful unsupported behavior, then pass the registered Anvil Windows build. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern platform portability, behavior contracts, scoped ownership and target evidence. ACC: `/home/oumy/work/emgine/config/man/anvil.md`. ToQ: preserve Node API behavior where Windows has an equivalent and return clear errors where it does not; ToT: keep shared JS operations above `cfg(unix)` / `cfg(windows)` backends, using Winsock and Windows process APIs rather than Unix `libc` symbols.

Evidence: Anvil operation `f5d0938f-a945-4b47-aa6b-252f22f0c3c4` compiled Pi commit `522b300` for `x86_64-pc-windows-msvc` and failed in `ops/dns.rs`, `ops/ipc.rs`, and `ops/process.rs` on Unix-only libc types/functions. The previous net socket handle boundary compiled far enough to expose these errors. Earlier Windows file, process, signal, console and UDP changes remain part of the same target test.

Risks: Windows process handle transfer and ancillary socket transfer have different ownership/duplication rules from `SCM_RIGHTS`; do not emulate them with integer casts. The dispatcher must avoid silently claiming Unix semantics. Files: `crates/pi-jsrt/src/ops/dns.rs`, `ipc.rs`, `process.rs`, and their platform modules. Validate with Linux `cargo check -p pi-jsrt`, then the same Anvil release-channel `pi-windows` build and OptiPlex1 native runtime checks.

Updated: 2026-11-05

Second Anvil target result: operation `c67ab9db-9fda-4c4c-9d72-f03f60838928` built published commit `fc1b1ed23058a150e88bec207d1335cfabeb421f` and reached `pi-jsrt`, then failed with 18 Windows compile errors. The diagnostics show incomplete cfg boundaries in `node.rs` signal reset and `ops/fs.rs` symlink code, plus errors in the new Windows signal/IPC/process implementations and a misplaced Windows API import (`DuplicateHandle` is in `Win32::Foundation`). This is a target compile failure, not evidence of runtime acceptance. Repair those concrete issues; validate Linux `cargo check -p pi-jsrt`, publish, and repeat the registered Anvil release build before OptiPlex1 runtime verification. Anvil VPM `vm-wgv2` records this operation.

Updated: 2026-11-05

Third Anvil target result: operation `0fe60fb7-058b-439f-a733-c7b480e964f3` compiled `pi-jsrt` for Windows, then found that three Pi app entry points import `pi_tui::terminal::ProcessTerminal`, which is exported only under `cfg(unix)`. The TUI has no Windows terminal implementation yet, so the runtime port alone cannot produce a usable interactive Windows executable. Add a native Windows console-backed `ProcessTerminal` with mode save/restore, virtual-terminal input/output, buffered key/protocol parsing, resize notification, and interruptible reader shutdown. Validate the target build, then exercise a real interactive startup/shutdown on OptiPlex1; compile-only evidence is insufficient.
