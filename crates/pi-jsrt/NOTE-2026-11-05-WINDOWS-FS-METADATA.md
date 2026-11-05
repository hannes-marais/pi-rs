2026-11-05

# Windows filesystem metadata lowering

Owner: Codex; EMgine VPM vm-1d9a. Outcome: compile Node fs stat/readdir on Windows and expose native timestamps, attributes and file type through the existing StatData contract. Architecture principles 3, 6, 13, 21, 24, 27, 30, 35 and 37 govern OS-specific lowering and target evidence. ACC: /home/oumy/work/emgine/config/man/anvil.md. ToQ: Node-compatible stats preserve the host filesystem's actual values where Windows provides them and do not fabricate POSIX identities; ToT: keep the shared StatData contract and isolate host extraction behind cfg-specific functions.

Files: crates/pi-jsrt/src/ops/fs.rs. Risks: Windows has no POSIX uid/gid, device special file types, mode bits, block counts or ctime semantics; leave unsupported fields explicitly zero and map readonly into permission bits rather than claiming POSIX ownership. Validate with Anvil's registered Windows build followed by native OptiPlex1 stat/readdir behavior; Linux check must remain clean.

Updated: 2026-11-05. The Anvil Windows build passed the filesystem metadata errors and now reports Unix-only child-process APIs. Next portability batch uses Windows process creation/termination and exit-code semantics; POSIX uid/gid and Unix extra-fd IPC must be reported as unsupported unless a real Windows equivalent is implemented.

Updated: 2026-11-05. The child process ExitStatus mapping now uses Windows' native process exit code with no fabricated Unix signal and the prior Unix signal mapping remains unchanged. Linux pi-jsrt check passes; Windows build is required to advance the remaining child-process and signal blockers.

Updated: 2026-11-05. Rust 1.97 marks Windows volume serial and file index metadata access unstable, so dev/ino are reported as zero until a stable handle-based implementation is added; ownership, block allocation and POSIX ctime are likewise unavailable.

Updated: 2026-11-05. File descriptors returned by fs.open are now runtime table IDs rather than host raw descriptors, allowing the same FsOp contract on pointer-sized Windows handles. Standard streams lower through duplicated Win32 handles; Windows chmod lowers permission bits to the native readonly attribute. Linux pi-jsrt check passes; Anvil Windows compilation is the next validation.
