2026-11-05
Updated: 2026-11-05

# QuickJS Intl.Collator parity investigation

VPM: emgine `vm-a0ks`, `vm-8x76`. Owner: Codex. The Windows test-runtime
selection remains provisional until the actual Maw JSON parity law passes on
OptiPlex1. Governing architecture principles: 3 (executable evidence), 13
(reproducible builds), 24 (stable interfaces), 27 (learn from failures), 30
(explicit target identity), 33 (portable platform lowerings), 35 (preserve
test coverage), and 37 (track before work). ToQ emphasizes reliability,
functionality, auditability, and causal depth; ToT favors reuse of the already
pinned ICU4X substrate when it meets the actual oracle.

## Question

Can the ICU4X version already pinned by the Pi workspace reproduce the
Foundation/Chromium key ordering required by Maw's generated web parity test,
including its numeric and case options, without adding a system-runtime
dependency or weakening the test?

## Evidence and plan

The first QuickJS candidate special-cased case-only ties and made `["a", "A"]`
return the required `-1`, but the unchanged full Maw law then failed at
`["a", "Aa"]` (actual `1`, expected `-1`). The collation's case-insensitive
primary order must be applied before prefix comparison, with case order as a
secondary weight. A second casefolded-codepoint candidate passed those two
pairs but failed at `["a", "😀"]` (actual `-1`, expected `1`). Unicode root
collation weights differ from codepoint order, so both approximations are
rejected. Implement or reuse a portable Unicode collation engine and rerun the
complete fixture oracle; do not publish either candidate.

The official Windows Anvil profile operation
`1fb11eea-2e2d-4d4e-8115-bb01b0474f1f` passed 132 Maw test binaries and then
failed the preserved `web_json_collation_matches_swift` law. The first pair
`["a", "A"]` returned `1` from Pi's QuickJS runtime, while the Swift 6.2.3
oracle expects `-1` for `caseFirst: "lower"`. The current JavaScript Collator
shim stores its options but delegates to QuickJS `String.prototype.localeCompare`,
which performs normalized code point ordering and does not implement locale
collation or `caseFirst`. This is direct evidence that the shim is not a valid
collation implementation. A one-pair case patch is not acceptance: the full
59,902-pair oracle and generated JSON parity law remain the gate.

The existing reference fixtures are in `rust/maw/tests/fixtures/` in the
emgine checkout: 59,902 ordered pair results across the base, long-number,
prefix, and Unicode corpora. An earlier ICU4X 2.1.1 candidate differed on 174
mixed-script cases; a naive numeric-boundary adjustment differed on 806.
The Pi workspace currently pins ICU4X 2.3.1 and already uses its compiled root
collator in `pi-tui`. This is a new candidate to measure, not evidence of
parity.

First run the exact fixture corpus against the pinned collator configurations
and report mismatches by corpus and input category. Then compare any candidate
that incorporates Foundation's documented ASCII digit-run handling against
the same fixtures and the actual `json_web_parity.mjs` acceptance case. Only
adopt an implementation that passes the full oracle and regression law. If
the ICU version still diverges, retain the runtime as unselected and continue
at the portable collation layer; do not special-case fixture values.

## Validation

The pinned ICU4X 2.3.1 root collator was replayed against all 59,902 fixture
pairs with tertiary strength and lower-case-first. With ICU numeric ordering
enabled, it mismatched 5,216 pairs (52 base, 3,410 long-number, 200 prefix,
1,554 Unicode). With ICU numeric ordering disabled, it mismatched 11,962
pairs (160 base, 6,212 long-number, 232 prefix, 5,358 Unicode). A scratch
ASCII-digit segmentation adapter also failed (9,210 mismatches); it is not a
candidate for adoption. Thus the newer pinned ICU version and simple option
mapping do not resolve the earlier ICU4X mismatch.

The actual Node/ICU plus Foundation's ASCII integer-run and UTF-16 prefix
rules previously matched all 59,902 pairs, but that result does not yet have a
portable ICU4X implementation. Next work must recover and port the verified
comparator semantics rather than tune options against a few samples. Keep
QuickJS unselected for the full Windows Maw gate until the unchanged Rust
`json_web_parity_laws` test passes through QuickJS on Linux and the native
Windows OptiPlex1 run passes through Anvil. These checks are distinct from a
cross-compiled Pi artifact and from Anvil profile build success.

## Risks

ICU/CLDR data version differences, non-transitive comparator behavior,
numeric-run semantics for non-ASCII decimal digits, and accidental changes to
deployed Pi sorting behavior. Keep the experiment isolated until evidence
supports a general-purpose implementation.
