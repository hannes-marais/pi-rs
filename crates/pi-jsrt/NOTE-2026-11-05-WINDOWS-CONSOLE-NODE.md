2026-11-05

# Windows process stdin and terminal mode

Owner: Codex; EMgine VPM vm-1d9a. Outcome: expose the Node stdin stream and raw terminal mode through standard Rust I/O and native Windows console modes, preserving the existing Unix termios path. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern portable host lowering and truthful target evidence. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: stdin reads retain retry/end/error semantics across redirected pipes and consoles; raw mode changes only console input echo/line/processed flags and restores the original mode. ToT: use `std::io::Read` for the shared stream and cfg-specific terminal mode adapters.

Files: crates/pi-jsrt/src/ops/process.rs. Validate with Linux Rust 1.97 check, registered Anvil Windows build and native OptiPlex1 console/redirected-stdin behavior.
