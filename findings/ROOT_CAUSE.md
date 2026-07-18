# Root-cause classification: adapter bugs vs. genuine dashu bugs

Every finding was reproduced **directly against the dashu API** (not through the
opendp-num adapter) in `examples/root_cause.rs`. The correct directed result is
derived from the exact rational value via `RBig` comparison — independent of dashu's
`to_f64` approximation tag — and, for transcendentals, the **raw `FBig` result is
inspected before `to_f64`** to separate the operation from the conversion.

Run: `cargo run --example root_cause --features all-backends`

## Headline

**Nearly all 15 findings are opendp-num *adapter* bugs, not dashu bugs.** They trace to
two adapter defects that share one missing primitive (a correct *exact-rational →
directed f64* rounding). The genuine upstream residue is **~2 findings**.

## Verified outcome (after the fix)

The adapter fix landed in `src/backend/dashu.rs`. `fuzz/verify_findings.py` replayed
every retained reproducer against the fixed adapter:

- **13 no longer reproduce (fixed):** DASHU-002, 004, 005, 006, 010, 011, 012, 013,
  014, 015, 016, 018, 019. Archived under `resolved-by-adapter-fix/`.
- **2 still reproduce (genuine upstream):** **DASHU-008** (dashu-int GCD panic) and
  **DASHU-007** (dashu-float has no correctly-rounded `log2`; the directed result is a
  sound but loose bound). These remain the active `findings/`.
- **DASHU-015** is a genuine dashu `to_f64` bug that the adapter now routes around;
  file it upstream with `examples/root_cause.rs` as evidence even though it no longer
  manifests through opendp-num.

`examples/verify_fix.rs` drives the adapter for each fixed case and asserts the
correctly-rounded result (all pass).

**Regression guard.** Re-fuzzing the rewrite against MPFR immediately surfaced two
new edge-case bugs it had introduced — signed-zero of a cancellation result
(`+0 + -0` under `Down` must be `-0`) and conversion overflow (must saturate to
`±inf`, not error). Both were fixed and added to `verify_fix.rs`. A follow-up
~4-minute, 4-worker slice on the now-fully-fixed `directed_binary` and
`conversions` targets produced no violations.

**Soundness caveat (transcendentals).** Correctly-rounded directed transcendentals
use Ziv-style precision doubling with input-magnitude-scaled start precision. This
is stability, not a rigorous directed-bound proof: for some inputs dashu's directed
mode can sit on the wrong side at insufficient precision. The differential fuzzer
against MPFR is the ongoing guard for this property; treat a long clean campaign,
not the unit checks, as the evidence of correctness here.

## Verdict table

| Finding | Op | Owner | Root cause / evidence |
|---|---|---|---|
| DASHU-002 | sub | **adapter** | FBig-arithmetic double-round; exact-RBig Down == MPFR (`0x7fe…ffe`) |
| DASHU-010 | div | **adapter** | native `1/tiny`=inf trips precheck (`dashu.rs:414`); correct Down = `f64::MAX` |
| DASHU-012 | add | **adapter** | FBig double-round; exact-RBig Up == MPFR (`0xbfe…ffe`) |
| DASHU-016 | div | **adapter** | FBig double-round; exact-RBig Up == MPFR (`0x000ff20c35575476`) |
| DASHU-018 | add | **adapter** | FBig mishandles −0; exact-RBig Up == MPFR (operand preserved) |
| DASHU-005 | expm1 | **adapter** | native `exp_m1`=inf trips precheck (`dashu.rs:285`); directed result representable |
| DASHU-019 | mul | **adapter** | FBig underflow → `-min_subnormal`; exact-RBig Down == MPFR (`+0`) |
| DASHU-006 | ln1p | **adapter** | **raw FBig@4096 < 2⁻¹⁰⁷⁴ (correct)**; `to_f64`+`from_down` mangles to `min_subnormal` instead of `+0` |
| DASHU-013 | expm1 | **adapter** | **raw FBig@4096 > 2⁻¹⁰²² (correct)**; conversion fails to round up |
| DASHU-011 | powi | **adapter** | **raw FBig sign positive (correct)**; `raw.to_f64()=Inexact(0.0,NoOp)` → `from_down`→`next_down_f64(0.0)`=`-min_subnormal` (crosses zero) |
| DASHU-004 | exp (f32) | **adapter** | same underflow/zero-boundary conversion class as 011/019 (adapter also has explicit error-catch hacks `dashu.rs:296-311`) |
| DASHU-007 | log2 | **dashu (limitation), low pri** | no correctly-rounded log2: `log2_bounds` and `ln/ln2` both give loose but **sound** upper bound; not tight `-1022` for `2⁻¹⁰²²` |
| DASHU-015 | int→f64 | **dashu** | `IBig::to_f64()` returns `Exact(-3.40…385e38)` for an **inexact** value (wrong-side). Genuine `to_f64` correctness bug — though the adapter can defend by not trusting the tag. |
| DASHU-008 | rational | **dashu** | dashu-int GCD allocation **panic** on valid input |

## The adapter fixes (all in `src/backend/dashu.rs`)

The two defects share one missing primitive:

- **A correct exact-rational → directed f64/f32 rounding.** Never trust `to_f64()`'s
  `Approximation` tag or `next_up/next_down` across zero. Use `to_f64()` only as a seed
  and re-derive by exact `RBig` comparison (`examples/root_cause.rs::round_directed`),
  which clamps correctly at `±0` and the subnormal boundary. Fixes the whole
  conversion/underflow cluster (004, 006, 011, 013, 015-symptom, 019) and the
  `next_down_f64(0.0) = -min_subnormal` sign crossing.

1. **Directed binary arithmetic** (`directed_binary!`): compute via **exact `RBig`** then
   the single directed rounding above — not `FBig`-with-mode arithmetic (which
   double-rounds). Proven == MPFR for 002/010/012/016/018/019. Uses dashu's own `RBig`;
   Malachite not required.
2. **Directed unary / transcendentals**: the `FBig` result is already correct at adequate
   precision — convert it via exact rational → directed rounding instead of the
   `to_f64`-tag path. Also raise working precision above target for the
   underflow-resolution cases.
3. **Drop the native `is_infinite()`/`is_nan()` prechecks** (`:241`, `:285`, `:414`, `:467`)
   that reject representable `±f64::MAX` results (005, 010).

## Genuine upstream residue (file after fixing the adapter + re-fuzzing)

- **DASHU-008** — dashu-int GCD panic on valid input. Clean, high-value. **File.**
- **DASHU-015** — `IBig::to_f64()` mislabels an inexact conversion as `Exact`. Crisp
  `to_f64` correctness bug. **File** (and defend against it in the adapter regardless).
- **DASHU-007** — no correctly-rounded `log2`; loose-but-sound. Low priority; likely a
  feature gap rather than a defect.

That is **~2 issues to file**, not 15.

## "Switch to Malachite" — answer

Not viable and not needed. Malachite is a 36-line exact-int/rational backend with **no
float surface** (no directed IEEE arithmetic, no transcendentals, no conversions), so it
cannot replace the operations these findings touch. The float findings are adapter
rounding bugs fixable with dashu's own `RBig`. Malachite's only real benefit is
sidestepping **DASHU-008** by routing rational reduction through it — optional and partial.

## DP-soundness priority (independent of owner)

Wrong-**sign** / wrong-**side** bounds break the directional guarantee and must be fixed
first: **DASHU-011, 019, 004** (wrong sign), **DASHU-006, 013** (wrong side of exact) —
all adapter conversion bugs. Loose-but-sound one-ULP bounds (002, 016, 007) are lower.
