2026-11-05

# Windows filesystem metadata lowering

Owner: Codex; EMgine VPM vm-1d9a. Outcome: compile Node fs stat/readdir on Windows and expose native volume/file identity, timestamps, attributes and file type through the existing StatData contract. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern OS-specific lowering and target evidence. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: Node-compatible stats preserve the host filesystem's actual values where Windows provides them and do not fabricate POSIX identities; ToT: keep the shared StatData contract and isolate host extraction behind cfg-specific functions.

Files: crates/pi-jsrt/src/ops/fs.rs. Risks: Windows has no POSIX uid/gid, device special file types, mode bits, block counts or ctime semantics; leave unsupported fields explicitly zero and map readonly into permission bits rather than claiming POSIX ownership. Validate with Anvil's registered Windows build followed by native OptiPlex1 stat/readdir behavior; Linux check must remain clean.
