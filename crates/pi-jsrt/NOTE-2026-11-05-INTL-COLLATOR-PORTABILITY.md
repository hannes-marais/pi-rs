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

## Next candidate: pin ICU4X 2.1.1 as the workspace collation provider

The 2.3.1 replay is materially worse than the earlier 2.1.1 candidate (5,216
versus 174 mismatches). Cargo refused a second version-alias dependency because
the workspace had already resolved the 2.3.1 package and its incompatible ICU
dependency graph. Pin the workspace to one 2.1.1 version and use it for a
platform-neutral `pi-jsrt` host op, with `sensitivity`, `caseFirst` and
`numeric` translated from the public Intl options. This also changes
`pi-tui`'s root locale ordering; rerun its existing collation and sort tests.
This is a measurement candidate, not acceptance: the mismatch corpus must be
retained and categorized, and the unchanged
`web_json_collation_matches_swift` law must pass all 59,902 pairs on Linux
before requesting another official Windows profile. If 2.1.1 still has 174
mismatches, characterize the exact pair categories before attempting a general
collation rule; no fixture-specific exceptions.

The candidate host interface is platform-neutral Rust and uses compiled ICU4X
data, so its behavior does not depend on an installed system ICU, Windows NLS
version, or Node. It preserves the option surface explicitly; it does not
establish native target acceptance.

The 2.1.1 host op compiles under the enrolled Rust 1.97 toolchain and makes
the first failing case (`["a","A"]`) pass. The unchanged Maw gate still fails
on 174 rows in the full 59,902-pair replay. Of those rows, 160 contain a
mathematical alphanumeric digit and 14 contain legacy-computing or other
number-like symbols. A Node 22 probe reports ICU 76.1 / Unicode 16.0; the
official ICU 76 release uses CLDR 46. ICU4X 2.1.1's bundled collator data says
it was generated from CLDR 48.0 / ICU 78.1rc. This makes collation-data drift a
plausible cause, but not yet a proven one. Keep the exact fixture witnesses
and strict verdict. Sources: ICU 76 release notes,
https://unicode-org.github.io/icu/download/76.html; installed
`icu_collator_data-2.1.1/README.md`.

ICU4X 2.0.0 was then replayed as the nearest stable data-generation candidate.
Its collator data is CLDR 47.0 / ICU 77, versus 2.1.1's CLDR 48.0 / ICU
78.1rc. It produced the exact same 174 mismatch witnesses (SHA-256 of the
sorted witness set `5d5b6c2cd54ce144994f150cdd26c7c7ed92079eee7f687a75de340ba76c171f`).
Thus the CLDR 47-to-48 data change does not explain these failures. The
2.0.0-beta1 collator and CLDR 46.0 / ICU 76 data were then built using the
upstream lock's compatible transitive versions. Its full 59,902-pair replay
still produced the same exact 174 mismatch witnesses as stable 2.0.0 and
2.1.1. Thus the data-generation version does not explain these rows. This
rules out the theory that newer ICU data reclassifies the math digits. The
next investigation is numeric comparison behavior at an identical digit
prefix followed by a digit-versus-letter divergence. Compare ICU4X's numeric
CE iterator and prefix handling to ICU4C 76's `CollationIterator` and
`CollationCompare`, preserve the 174 witnesses, and keep the full-corpus gate
strict. The beta dependency was diagnostic only; production Pi stays on stable
ICU4X until a passing portable comparator is proven. VPM `vm-a0ks` tracks
implementation; `vm-8x76` tracks native test-runtime acceptance.
Formal-methods principles 2 (retain replayable witnesses), 8 (replay recorded
verdicts), and 12 (check running behavior against the oracle) govern this
investigation.

## Prefix-context differential

Replayed the retained Anvil `appcode-json-collation-20261029` Rust comparator
probe against the same 59,902 pairs. Its general UTF-16 prefix / ASCII numeric
run adapter plus ICU4X numeric collation leaves 12 mismatches, down from 174.
As a controlled variation, starting ICU4X comparison at the first raw UTF-16
mismatch instead of retaining the prior collation context produces 1,430
mismatches. This rules out dropping equal prefix context as a safe repair and
confirms that a correct fix must preserve ICU's context while resolving numeric
CE behavior for supplementary decimal characters. The 12-row residual corpus is
in `/home/oumy/anvil-build-data/anvil/fdevbox/work/evidence/appcode-json-collation-20261029/icu4x-mismatch-math.jsonl`;
the 1,430-row variant was replayed from `/tmp/icu-collation-replay`.
Headline lineage edges are in
`emgine:src/anvil/build/pi-jsrt-windows-module-path-2026-11-05.braid` and are
surfaced by `braidql briefing --host ia-playground -e main`. No candidate was
adopted; the full corpus remains the gate.

## ICU 76 pairwise behavior

Direct Node 22.15.1 / ICU 76.1 `Intl.Collator('und', {numeric:true,
sensitivity:'variant', caseFirst:'lower'})` returns `compare('2a','21') = -1`,
`compare('21','2𝟙') = 0`, and `compare('2a','2𝟙') = 1`. This is not
substitution-consistent with the equality result: `2𝟙` and `21` compare equal,
but compare on opposite sides of `2a`. Keep the pairwise oracle exact and do
not assume transitivity when analyzing a total sort result; runtime parity must
reproduce the pair function. Command: `node -p` with the three comparisons in
the same process. The result is transmitted by the braid headline. This does
not establish that Swift's `sortedKeys` accepts or preserves the same order.

## Risks

ICU/CLDR data version differences, non-transitive comparator behavior,
numeric-run semantics for non-ASCII decimal digits, and accidental changes to
deployed Pi sorting behavior. Keep the experiment isolated until evidence
supports a general-purpose implementation.

## Direct ICU 76 configuration audit

Built a temporary N-API addon over Node 22.15.1's exported ICU 76.1 C API.
The probe reports an en-US valid locale, tertiary strength, lower-first,
numeric on, and normalization on, matching V8's `CreateICUCollator` setup
(which enables normalization unconditionally; source:
https://chromium.googlesource.com/v8/v8/+/790b2d341c5fb0cc6438f8af1b979bff63e55be1/src/i18n.cc).
Across all 59,902 ordered Maw pairs, Node Intl and direct
`ucol_strcoll` over UTF-16 agree on every sign. Direct `ucol_strcollUTF8`
differs from Node on 238 pairs. Thus the earlier direct-ICU discrepancy is an
ICU entry-point difference, not evidence that V8 adds a custom comparison
rule. The V8 fast comparison path is unavailable for this explicit-options
collator. The headline result is in the EMgine braid and Labbook experiment
`exp-qn49` / evidence `evk-60z9`.

This does not make direct ICU UTF-16 a candidate for Maw parity: Node and
direct UTF-16 still differ from the Swift expected pair corpus on 4,978 of
59,902 rows. On the preserved 12-row ICU4X residual set, Node/UTF-16 matches
Swift on all 12, while UTF-8 differs on four. The remaining eight residual
rows therefore still isolate ICU4X behavior; continue with the Unicode
numeric-weight witnesses and keep the complete Swift corpus as the gate. The
12 directed residual rows are six inverse pairs. Four rows (two inverse pairs)
are math-Nd versus math-Nd comparisons where ICU's UTF-8 and UTF-16 APIs also
diverge. The other eight rows (four inverse pairs) are mathematical or
segmented Nd versus Greek capital omega: Node and both ICU76 APIs match the
Swift sign, but ICU4X returns the opposite sign. In the preserved comparator,
all four pairs enter the `astral_digit` split branch after a shared decimal
prefix. Investigate ICU4X's numeric-primary weights and this split boundary;
do not change it until the full corpus replay confirms improvement.
Governing principles: architecture 3, 13, 24, 30, 35, 37; formal-methods 2,
8, 12. The probe is diagnostic only; no runtime candidate is accepted.
