2026-11-05

# Windows TCP handle boundary

Owner: Codex; EMgine VPM `vm-1d9a`. Outcome: make Pi's `node:net` TCP handle ownership compile on Windows while preserving native socket handles and Unix descriptor behavior. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern portable semantics, explicit platform boundaries, evidence, and tracking. ACC: `/home/oumy/work/emgine/config/man/anvil.md`. ToQ: retain socket ownership and lifetime across Tokio/std conversions; ToT: use Rust's platform socket-handle traits with a shared handle type and explicit ownership conversions.

Assumptions: Windows `RawSocket` values fit the existing JS numeric handle transport; cross-process cluster socket transfer remains dependent on Windows-specific duplication semantics and must be validated on OptiPlex1. Risks: ownership bugs can close a socket twice or leak it; JS handle passing may need a follow-up abstraction after compile succeeds.

Files: `crates/pi-jsrt/src/ops/net.rs`. Validation: Linux `cargo check -p pi-jsrt`; official Anvil Windows release-channel build; then native OptiPlex1 checks for listen/connect/adopt and cluster handle transfer.
