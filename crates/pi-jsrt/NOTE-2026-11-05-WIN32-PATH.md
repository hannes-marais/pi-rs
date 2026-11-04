2026-11-05

# Node-compatible Windows paths

Owner Codex; EMgine VPM vm-hbjr under vm-a0ks. ACC /home/oumy/work/emgine/config/man/anvil.md. Architecture principles 3, 6, 13, 21, 24, 27, 29, 30 and 35 govern reuse, generic semantics, target fidelity and evidence. ToQ: path.win32 handles drive, UNC, separator, relative, parsing and formatting rules on every host, and the default path module follows the host; ToT: reuse the audited Node path algorithm and preserve the existing POSIX interface.

The current shim aliases win32 to POSIX, so generated code misinterprets native Windows paths even on a correctly ported runtime. Adapt the MIT-licensed Node v22.15.1 Windows path algorithm to Pi's existing glue and runtime cwd; preserve provenance and license. Capture a fixed reference corpus from Node for drive/UNC/device/dot and malformed inputs so Pi tests do not require Node at runtime. Test Linux and, once the Windows host APIs compile, repeat on OptiPlex1 and macOS. Existing matchesGlob has separate incomplete semantics; preserve it and track that gap rather than claiming complete parity. No other Node APIs or test gates may be stubbed or skipped.

Updated: 2026-11-05. Adapted Node v22.15.1's MIT-licensed Windows path algorithms and preserved its copyright/license in the glue. The Pi QuickJS runtime passed 1,147 fixed Node-reference cases on Linux, including drive/UNC/device paths, traversal, parsing, formatting and invalid arguments; the existing eight-test Node compatibility suite also passed. Default host selection is implemented, but native Windows and macOS execution remain unverified because the Windows host runtime does not yet compile and no macOS target run was performed. `matchesGlob` retains the prior simplified matcher and is tracked separately in vm-p62g; complete path parity is not claimed.

Updated: 2026-11-05. The prior `matchesGlob` gap is addressed by the separately tracked Node-compatible bundled matcher; see NOTE-2026-11-05-PATH-GLOB.md. Native Windows/macOS runtime checks remain required.
