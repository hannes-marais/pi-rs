2026-11-05

# Windows UDP lowering

Owner: Codex; EMgine VPM vm-1d9a. Outcome: preserve Node dgram bind/connect/disconnect behavior on Windows using socket2/Tokio and Winsock, with explicit reporting for reusePort where Windows has no SO_REUSEPORT contract. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern shared semantics, declared APIs and target evidence. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: UDP peer state and bind options follow host capabilities without raw Unix descriptors; ToT: use Tokio's portable connect and a narrowly scoped Winsock disconnect lowering.

Files: crates/pi-jsrt/Cargo.toml and src/ops/dgram.rs. Validate by Linux Rust check and Anvil windows/amd64 build; OptiPlex1 UDP behavior remains a later acceptance gate.

Updated: 2026-11-05. The Windows branch now uses Winsock AF_UNSPEC disconnect and rejects reusePort explicitly; the Unix socket2/raw-fd path remains cfg(unix). Linux pi-jsrt check passes; Windows API compilation and runtime remain unverified pending the next Anvil run.
