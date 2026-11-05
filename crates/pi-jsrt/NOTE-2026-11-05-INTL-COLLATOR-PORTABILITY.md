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

## Next implementation candidate

Port the already replayed general comparator from the preserved Anvil probe
into the platform-neutral `pi-jsrt` host op: compare ASCII digit runs as
unbounded integers with a deterministic length tie-break, retain UTF-16
common-prefix context for ICU4X fallback, and use Unicode General_Category
Nd data rather than fixture-specific code points for the supplementary-digit
boundary rule. Keep all `Intl.Collator` sensitivity, caseFirst, and numeric
options intact. First require the unchanged 59,902-pair Maw law to improve
from the current direct-ICU4X result without reducing fixture coverage, then
address every remaining row with a categorical rule and rerun that same full
law on Linux before requesting native Windows acceptance. This is one
portable host implementation used by the same QuickJS runtime on Linux,
Windows, and macOS; it is not a Windows-specific substitute. This plan is
governed by architecture principles 3, 13, 24, 30, 33, 35, and 37, and formal
methods principles 2, 8, and 12.

The first port compiled, but its full QuickJS/Linux Maw replay failed with
348/59,902 mismatches, worse than direct ICU4X's 174. A controlled replay
restored the probe's exact two astral-digit ranges while retaining unbounded
ASCII-run comparison; the full corpus then returned to 12 mismatches. The
generalization to all astral Nd characters caused the 336 additional failures;
the unbounded numeric-run comparison did not change the observed residual
count. The remaining rows are the same six inverse pairs previously isolated
as math/segmented digits versus Greek omega or math-Nd comparisons. This
candidate compiles on Linux but is not an acceptance candidate while those 12
rows remain. Keep the full corpus gate and investigate a rule that explains
all six pair classes without extending a fixture-specific code-point list.

## Residual split-path isolation

The 12 directed failures comprise four rows in math-Nd versus math-Nd pairs,
where ICU76 UTF-8 and UTF-16 entry points also disagree, and eight rows in
supplementary-Nd versus Greek capital omega pairs, where Node/Swift agree and
ICU4X disagrees. The latter class enters the special split-at-digit-boundary
path. Next disable only that split while retaining localized UTF-16 prefix
context and replay all 59,902 pairs. This is a diagnostic variation, not an
accepted behavior change. Governing architecture principles 3, 13, 24, 30,
33, 35, 37 and formal-methods principles 2, 8, 12 apply.

Disabling only the split path replayed to 174/59,902 mismatches, the same
count as direct ICU4X. Therefore the split path is responsible for a large net
improvement (162 fewer failures) and cannot be removed wholesale, even though
it leaves eight omega-related residual rows. Restored the 12-mismatch
experimental candidate in the working tree. This confirms the next fix must
refine the boundary decision using a general collation/numeric-run signal,
not remove the path or widen its Unicode-category predicate.

## Native UTF-16 API differential

The candidate scans UTF-16 but converts fallback suffixes to Rust `str` with
`String::from_utf16_lossy` and calls ICU4X `Collator::compare`. ICU4X 2.1.1
also exposes `Collator::compare_utf16`, which preserves the JS input domain
and avoids lossy conversion. Test this API on the unchanged 59,902-pair gate
before further tuning the split rule. This is a portable ICU4X API available
on Linux, Windows, and macOS; architecture principles 3, 13, 24, 30, 33, 35,
37 and formal-methods principles 2, 8, 12 govern the experiment.

The native UTF-16 API preserves the JS domain but the first full replay still
reports the same 12 pairs. An isolated ICU4X 2.4.0 direct `compare_utf16` probe
reports 5,216/59,902 mismatches, versus 174 for direct ICU4X 2.1.1; do not
upgrade to 2.4.0 for this fix.

The JS shim's `resolvedOptions()` and locale support report `en-US`, while
`CollatorPreferences::default()` currently selects root/`und` data. An isolated
ICU4X 2.1.1 direct `compare_utf16` probe with `en-US` still gives 5,216 corpus
mismatches, the same as ICU4X 2.4.0 root, so locale alone is not a repair. Set
the candidate's explicit locale to match its web API contract and replay the
full adapter gate; keep locale support pinned to `en-US` until broader locale
selection is implemented.

The unchanged full Maw replay using ICU4X 2.1.1 `compare_utf16` still fails
the same 12/59,902 rows as the UTF-8 `compare` candidate. Thus the lossy
UTF-16-to-String conversion was not causal for these residuals. An isolated
candidate retains the useful split-path adapter and native UTF-16 suffix
comparison, but remains unaccepted. Continue investigating ICU4X numeric
collation behavior against the six inverse residual pairs.

An isolated 2.1.1 differential over all 59,902 pairs found 374 cases entering
the current astral-digit split branch. Comparing full input versus split
suffix against Swift expected signs: full input was correct on 140, split on
366, both on 132, neither on 0. Thus split alone helps 234 rows and harms
eight. All eight full-only wins are supplementary Nd versus Greek capital
omega; the four remaining pair rows are two Nd-versus-Nd pairs where both
whole and split ICU4X signs are wrong. These counts account for the 12
residual rows and show the failure mechanisms are distinct: boundary choice
for Greek omega, and cross-script numeric-run ordering for Nd-versus-Nd.
Next derive the choice from collation/numeric data instead of code-point
ranges, then handle mixed-script numeric runs without enumerating characters.

Unicode property enumeration shows the two legacy astral ranges are both
`General_Category=Nd` and `Script=Common`. ICU4X property data also contains a
third astral Common-script Nd range, U+1CCF0..U+1CCF9. Replace the two literal
ranges with the semantic conjunction `astral && Nd && Script=Common`, then
replay the full corpus. This tests a portable Unicode property boundary and
will also reveal whether the third range occurs in the fixture corpus.

The `astral && Nd && Script=Common` candidate compiles and leaves the corpus at
12/59,902 mismatches, the same as the two literal ranges. The additional
U+1CCF0..U+1CCF9 range does not alter the observed outcome. The four Nd-vs-Nd
residual rows are missed because only ASCII/ASCII starts enter manual numeric
run comparison. Next compare Unicode decimal runs by their ICU4X numeric
collation value, deriving each scalar's digit value through the same collator
instead of code-point arithmetic; retain a per-scalar cache and rerun the full
law to catch unintended changes.

The first generalized decimal-run implementation compiles but regresses the
full law to 182 mismatches. Equal-value mixed-script runs were given an
ASCII-style run-length tie-break, which conflicts with the frozen Swift signs
for rows such as `02` versus fullwidth `２`. Refine the experiment: retain the
existing length tie-break only when both runs are ASCII; for runs containing
non-ASCII digits, use the derived numeric value only when the values differ,
otherwise leave equality and subsequent context to ICU4X. Replay all pairs.

That narrower tie-break replay returns 174 mismatches. The implementation
still consumed equal non-ASCII digit runs before ICU4X fallback, changing the
prefix context and suppressing the useful split behavior. Refine again: for
equal non-ASCII runs, fall back from the existing localized prefix without
advancing either cursor; manually return only when their derived numeric
values differ. Preserve ASCII-run consumption and tie behavior.

The second revision still reports 174: routing every ASCII/ASCII run through
collator-derived digit-value extraction altered the existing prefix behavior.
Keep the established ASCII-run path byte-for-byte. Add a narrow numeric
correction only when both current scalars are non-ASCII Nd; compare their full
contiguous Nd runs by values from ICU4X, return only for unequal numeric values,
and otherwise fall through without consuming the run. This isolates the four
Nd-vs-Nd residual rows without changing mixed ASCII/Unicode equal-run behavior.

The narrowed candidate reports 114 mismatches. Inspection of the captured rows
shows Unicode-run parsing also starts when the two current Nd scalars are
identical, causing it to compare unequal-length runs beyond a shared digit
prefix (for example Arabic 2 versus Arabic 2 followed by mathematical 1).
Require the current raw UTF-16 units to differ before comparing Unicode run
values; identical shared digits must stay in the prefix context. Replay the
full corpus again.

The scalar-unit guard reduces the result to 24 rows but still leaves the
original twelve and adds six new inverse pairs. The diagnostic shows that the
scan can land on the low surrogate of two differing supplementary scalars,
where the current scalar decoder returns `None`; it also applies the new
numeric correction to script-specific Nd runs. Make the comparison loop
advance and inspect complete Unicode scalars, preserving the old localized
boundary after shared supplementary scalars. Apply value comparison only when
both distinct current scalars are non-ASCII `Nd` with `Script=Common`, then
replay the same gate.

## Mixed digit/non-digit boundary hypothesis

The eight remaining corpus rows are four inverse pairs where one string
continues a decimal run with an astral Common-script digit and the other ends
that run at Greek capital omega. The current astral-digit split compares only
the differing suffix and loses the numeric context accumulated in the
preceding shared Nd prefix. Test a narrow correction at this boundary: when
both preceding scalars are decimal digits and exactly one current scalar is a
decimal digit, scan each entire contiguous Nd run and compare its digit values
using ICU4X-derived values. Return only if the numeric values differ; retain
the existing fallback for equal values and all other divergence types. This
tests a numeric-token boundary rule rather than enumerating the omega fixture.
The full 59,902-pair Maw gate is the falsifier. Architecture principles 3,
13, 24, 27, 30, 33, 35, and 37 govern the implementation and evidence.

The first implementation of this hypothesis returned the comparison of the
entire surrounding Nd runs whenever only one current scalar was a digit. After
building the changed `pi` executable (rather than reusing the previous binary)
the full gate regressed from 8 to 170 mismatches, so the experiment was
reverted. The failure shows unequal whole-number values do not determine
Foundation's ordering for every digit/non-digit boundary; contextual collation
must remain authoritative outside a narrower, evidence-backed case. The
previous 8-row candidate is restored in source; rebuild it and replay the full
gate before claiming the baseline is recovered.

## Greek letter plus numeric continuation diagnosis

A raw ICU4X `compare_utf16` replay of the eight omega witnesses shows that
the full localized suffix has the Swift sign, while truncating at the astral
digit boundary reverses it. A fixture-wide categorical scan of the split
boundary cases found the eight split-only failures in one structural class:
the non-digit side is a Greek uppercase letter immediately followed by an
Nd scalar. That property cohort contains ten directed rows; full comparison
is correct on all ten, and split comparison is correct on only the two rows
where both methods agree. The other split boundaries still supply the net
corpus improvement, so disabling the split generally is rejected.

Test one semantic exception to the split rule: when the non-digit scalar is
`Script=Greek`, `General_Category=Uppercase_Letter`, and the next scalar on
that side is `Nd`, keep the full localized suffix comparison. This derives
the condition from Unicode and token context rather than enumerating omega.
Run the exact 59,902-pair gate; any regression falsifies the refinement.
Governing principles remain architecture 3, 13, 24, 27, 30, 33, 35, and 37.

The refinement passes the complete Maw gate using the rebuilt Pi executable:
`MAW_JS_RUNTIME=/home/oumy/work/pi-rs/target/debug/pi
MAW_JS_RUNTIME_ARG=__pi_jsrt_node__ cargo +1.97.0 test --offline --locked
-p maw --test json_collation_web_laws`. Result: 59,902/59,902 Swift pair
comparisons plus the generated Web JSON cases pass. The gate first failed at
170 when an overly broad whole-number comparator was tried; that implementation
was reverted before this refinement. This is Linux candidate evidence only.
The source still needs publication, Anvil's pinned Windows build/test, and
native OptiPlex1 acceptance before it can be selected for deployment.
