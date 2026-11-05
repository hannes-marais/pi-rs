2026-11-05

# Native Windows process terminal for Pi

Owner: Codex; EMgine VPM `vm-wgv2` (Windows Anvil build). Outcome: provide the same interactive Pi TUI entry path on Windows through a native console terminal implementation. Principles 3, 6, 21, 24, 27, 30, 35 and 37 govern platform behavior, API contracts, target evidence and delivery. ACC: `/home/oumy/work/emgine/config/man/anvil.md`. ToQ: preserve the TUI `Terminal` behavior and reliably restore console state on shutdown; ToT: lower console mode, input waits and screen dimensions to Windows Console APIs while keeping sequence parsing and terminal behavior in the shared TUI layer.

Evidence: Anvil operation `0fe60fb7-058b-439f-a733-c7b480e964f3` compiled `pi-jsrt` for `x86_64-pc-windows-msvc`, then failed at three Pi app imports because `pi_tui::terminal::ProcessTerminal` is exported only on Unix. Existing `ProcessTerminal` includes raw input, bracketed paste, Kitty/modifyOtherKeys negotiation, output protocol handling, size tracking, and a wakeable reader. Windows has no equivalent implementation, so adding only runtime OS calls cannot yield a usable interactive executable.

Plan: add the Windows `ProcessTerminal` export and console-backed implementation. Save/restore input and output modes; enable virtual-terminal input/output when attached to a console; read UTF-8/VT input through an interruptible console wait; feed the existing `StdinBuffer`; preserve protocol negotiation and callbacks; poll dimensions for resize; wake and join the reader during stop. Retain sensible redirected-stdio behavior. Add `windows-sys` only for the APIs needed by this implementation.

Risks: Windows console VT input and handle signaling differ from POSIX polling; redirected input may not be waitable. Do not claim target acceptance until both the exact Anvil Windows release build passes and Pi starts, accepts keyboard input, resizes, and restores console state on OptiPlex1. Source file: `crates/pi-tui/src/terminal.rs`; dependency declaration: `crates/pi-tui/Cargo.toml`.

Validation: Linux `cargo check -p pi-jsrt` passed before this new finding. Next run workspace Linux compile, publish, repeat the exact registered Anvil Windows release build, then native OptiPlex1 acceptance.

Implementation update: `terminal_windows.rs` now exports a Windows `ProcessTerminal` backed by Windows Console APIs. It saves/restores console modes, enables VT processing and input on attached consoles, sends Kitty/DA and program-status queries, reuses `StdinBuffer` and shared sequence parsing, polls console dimensions, writes progress/status output, and uses a stop event to wake and join its reader. Linux command `rustup run 1.97.0 cargo check --offline -p pi-tui -p pi-coding-agent` passed. Redirected stdin does not start the console reader; env-based dimensions remain the fallback. The Windows build and native terminal lifecycle are still pending.

Updated: 2026-11-05

Anvil operation `532f966b-267f-4658-a06f-c74e99525e88` compiled the new Windows TUI module and found one error at reader-thread creation: Windows `HANDLE` is a raw pointer and is not `Send`. Keep the handle stored as an integer token across the thread boundary and cast to `HANDLE` only inside the Windows reader.

Updated: 2026-11-05

Anvil operation `0704ac13-1dcf-4f94-93c9-42e74af81451` confirmed the first HANDLE fix was incomplete: casting inside the `move` closure still caused the raw pointer to be captured. Convert to `isize` before constructing the closure so only the integer crosses the thread boundary.
