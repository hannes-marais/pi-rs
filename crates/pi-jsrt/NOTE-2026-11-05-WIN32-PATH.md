2026-11-05

# Node-compatible Windows paths

Owner Codex; EMgine VPM vm-hbjr under vm-a0ks. ACC /home/oumy/work/emgine/config/man/anvil.md. Architecture principles 3, 6, 13, 21, 24, 27, 29, 30 and 35 govern reuse, generic semantics, target fidelity and evidence. ToQ: path.win32 handles drive, UNC, separator, relative, parsing and formatting rules on every host, and the default path module follows the host; ToT: reuse the audited Node path algorithm and preserve the existing POSIX interface.

The current shim aliases win32 to POSIX, so generated code misinterprets native Windows paths even on a correctly ported runtime. Adapt the MIT-licensed Node v22.15.1 Windows path algorithm to Pi's existing glue and runtime cwd; preserve provenance and license. Capture a fixed reference corpus from Node for drive/UNC/device/dot and malformed inputs so Pi tests do not require Node at runtime. Test Linux and, once the Windows host APIs compile, repeat on OptiPlex1 and macOS. Existing matchesGlob has separate incomplete semantics; preserve it and track that gap rather than claiming complete parity. No other Node APIs or test gates may be stubbed or skipped.

Updated: 2026-11-05. Adapted Node v22.15.1's MIT-licensed Windows path algorithms and preserved its copyright/license in the glue. The Pi QuickJS runtime passed 1,147 fixed Node-reference cases on Linux, including drive/UNC/device paths, traversal, parsing, formatting and invalid arguments; the existing eight-test Node compatibility suite also passed. Default host selection is implemented, but native Windows and macOS execution remain unverified because the Windows host runtime does not yet compile and no macOS target run was performed. `matchesGlob` retains the prior simplified matcher and is tracked separately in vm-p62g; complete path parity is not claimed.

Updated: 2026-11-05. The prior `matchesGlob` gap is addressed by the separately tracked Node-compatible bundled matcher; see NOTE-2026-11-05-PATH-GLOB.md. Native Windows/macOS runtime checks remain required.

Updated: 2026-11-05. Anvil operation `07aab951-724b-426e-a467-e14b6f82851b`
ran Maw's preserved sidecar test on OptiPlex1 with Pi's QuickJS Node-compat
mode and failed with `ERR_MODULE_NOT_FOUND` for an absolute drive path. The
resolved Anvil target plan includes `rust/maw/tests/anvil_sidecar.mjs` among
the staged execution inputs, and Windows extraction completed; the remaining
fault is Pi `loader.rs::resolve_spec`, which recognizes only slash-leading
absolute paths before falling through to relative or package resolution. Use
host-native `Path::is_absolute()` at that boundary so `Z:/...` resolves as a
file on Windows while POSIX behavior stays unchanged. Verify the mixed
separator reproduction in a Windows-gated loader test, run focused Linux
runtime tests, and rerun the official Anvil profile build to execute the same
Maw test on OptiPlex1. This host-loader correction is governed by EMgine
architecture principles 3, 6, 13, 21, 24, 27 and 30; the open work is
`vm-hbjr` / `vm-8x76`. No system Node executable is part of this path.

Updated: 2026-11-05. After commit `5001574` fixed host-native absolute module
resolution, the same official Anvil profile operation
`0f0b1429-5705-4781-8254-6c3a1fe327fb` advanced the preserved Maw sidecar test
past module loading. It now fails in `js/node/misc.js::pathToFileURL`: the
native `C:\\Users\\...` path is encoded as `file://C:%5CUsers...`, which
`URL` rejects with `ERR_INVALID_URL`. The earlier diagnosis was incomplete;
source staging and absolute module lookup are functioning. Add host-aware
Windows file URL conversion and regression coverage, preserving POSIX URL
behavior; then rerun the same full Anvil profile build on OptiPlex1. The
verification remains the Pi QuickJS runtime, not a system Node executable.

Updated: 2026-11-05. The next official Anvil run
`6f1eee23-0afb-4220-a7c5-e5ce8b089e2e` used the pushed
`pathToFileURL` correction and passed Maw's
`anvil_sidecar_emits_and_executes_its_complete_offline_closure` test on
OptiPlex1. It then failed `json_reader_matches_the_replaced_reader` because
`node:fs::toPath` converts a file URL with only `decodeURIComponent(url.pathname)`;
for `file:///Z:/...`, that leaves `/Z:/...`, which Windows rejects (EIO 123).
The current `node:url.fileURLToPath` helper also returned the raw `/Z:/...`
pathname and rejected UNC hosts. The correction now performs Windows drive and
UNC conversion there and routes fs URL inputs through that shared helper. The
Linux node compatibility test passed with a file URL read round trip and
unchanged POSIX URL output. The Windows drive assertions still require the
next official full profile run on OptiPlex1. Operation
`6f1eee23-0afb-4220-a7c5-e5ce8b089e2e` failed overall; no installation was
applied.
