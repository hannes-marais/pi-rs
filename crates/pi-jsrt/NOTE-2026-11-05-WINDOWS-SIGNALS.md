2026-11-05

# Windows process signal lowering

Owner: Codex; EMgine VPM vm-1d9a. Outcome: deliver Windows console Ctrl-C/Ctrl-Break as Node-compatible SIGINT/SIGBREAK to the existing JS signal event channel. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern cross-platform semantics, truthful supported-signal boundaries and target proof. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: supported Windows control events reach every subscribed Pi runtime without performing JS work in the OS callback; ToT: a bounded event queue and dedicated dispatcher feed the same internal signalEvent hook used by Unix.

Files: crates/pi-jsrt/src/ops/signal_windows.rs and ops/mod.rs. Windows supports SIGINT and SIGBREAK subscriptions from console control events; other Unix signal numbers return EINVAL rather than pretending to install handlers. Validate with Anvil's Windows build and a native OptiPlex1 Ctrl-C/Ctrl-Break runtime check; Linux continues using its existing sigaction/pipe implementation.
