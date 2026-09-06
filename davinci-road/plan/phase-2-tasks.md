# Phase 2 — Task contracts

> [!NOTE]
> The 22 per-task contracts for [Phase 2 — Disegno and the Pass Manager](./phase-2.md) — Deliverable / Steps / Acceptance / Deps / Non-goals for P2-1 through P2-20. They live beside the phase file rather than inside it because the contracts alone exceed the repository's 350-line source-length budget (`tools/moon/cmd/source_file_lengths --max-lines 350`, which plan files are not exempt from). The phase-level record — what the re-cut changed, the phase-1 carry-ins, the TODO index and the exit gate — stays in [phase-2.md](./phase-2.md), and that index is where a task's box gets checked; the **Steps** sub-checkboxes are here.

## P2-1 — `vize_davinci` core types

**Landed 2026-08-19** — full record: [phase-2-records/p2-1.md](./phase-2-records/p2-1.md).

**Deliverable:** the id / side-table / diagnostic substrate every later stage keys on, in the existing P0-10 crate, which today declares exactly one module (`folio`) and depends only on `vize_carton`.

**Steps:**

- [x] `crates/vize_davinci/src/id.rs`: `NodeId(u32)` newtype — `Copy`, `Eq`, `Hash`, with a niche so `Option<NodeId>` stays 4 bytes (`NonZeroU32` or a reserved sentinel — pick one and record why in the PR) _(`NonZeroU32`; the reasoning is in the type's own docs)_
- [x] `crates/vize_davinci/src/side_table.rs`: `SideTable<T>` keyed by `NodeId`, with the residency decision recorded explicitly and the densification trigger documented rather than densified _(sparse `vize_carton::FxHashMap<NodeId, T>` only; the dense arena form and its three-condition trigger are written down, not built)_
- [x] `crates/vize_davinci/src/diagnostic.rs`: the unified `Diagnostic` — `vize_carton::Span`, stage-of-origin, structured parts, and a witness slot (empty until P4-6). Message text is **owned**, the deliberate P1-10 exception, so a diagnostic survives `Allocator::reset`
- [x] Node-size `const` asserts on all three _(with a **deviation recorded**: only the pointer-containing figures carry the `#[cfg(target_pointer_width = "64")]` guard — see the record)_
- [x] `'static` assertion on `Diagnostic` (the P1-11 arena/cache contract, enforced the same way `SfcCompileResult` and the batch cache types are)
- [x] `#![no_std]` + `extern crate alloc;` held; rustdoc per public type

**Acceptance:** `cargo test -p vize_davinci` green (TS-1) — 20 new unit tests, 31 total; the size asserts and the `'static` assertion compile (a violation is a compile error, not a test failure); `cargo build -p vize_davinci --target wasm32-wasip2` green; TS-11 empty, **proved mechanically** rather than argued — `cargo tree -i vize_davinci --workspace` lists no reverse dependencies, so nothing on any compile path can observe these types; TS-13 green on the new tests, with no allowlist entry added (the allowlist only shrinks). **Deps:** none (phase-1 exit). **Non-goals:** replacing `vize_relief::CompilerError` at its call sites — the single-diagnostics-channel convergence that structurally ends dual assembly is P4's; densifying the side table; the S2 ops themselves (P2-5a).

## P2-2 — Pass manager

**Landed 2026-08-19** — full record: [phase-2-records/p2-2.md](./phase-2-records/p2-2.md).

**Deliverable:** `crates/vize_davinci/src/pass/` — pipelines as const data with SIL-style classification and build-time fusion grouping. This is greenfield: `PassObserver`, `PassManager` and a pass-`Pipeline` type have **zero** occurrences in `crates/` today.

**Steps:**

- [x] pass module root: pass description as **const data** — no registry of trait objects, and dispatch resolved per pipeline, never per node (performance guardrail 1) _(`src/pass.rs`; the crate has no `mod.rs` anywhere, and ordinary `mod` declarations discover the sibling `pass/` tree)_
- [x] Classification enum `PassKind { MandatoryDiagnostic, MandatoryLowering, Optional }` (SIL import). Mandatory passes are unfusable barriers, run at every optimization level, and are the only passes that perform the raw→canonical transition _(the barrier law is asserted in the `const fn` constructor, so a violating `const DESC` does not compile — **verified by compiling one**)_
- [x] `Raw<S>` / `Canonical<S>` wrapper types so the transition is type-level: an optional pass cannot produce `Canonical<S>` because it cannot name the constructor _(private field + a `produce::<P>` that forces a per-monomorphization `const` assertion; the optional case is a compile error, **verified by compiling one**)_
- [x] `Fusability { Fusable, Barrier }`; fusion computes the **preserved-set intersection at build time** (`const fn`), so a grouping regression is a compile error rather than a runtime surprise _(every planning query is a `const fn`; the fixture plan is pinned in `const` items)_
- [x] pipeline syntax `s2(a,b),s2-to-s3(c)` parsed into the same const shape for `davinci-opt --pipeline`; grammar in the module docs with every error message spelled out _(`src/pass/pipeline.rs`; ten documented rejections, each asserted on its full rendered message)_

**Acceptance:** `cargo test -p vize_davinci` green (TS-1) — 61 tests, 25 new here — covering: pipeline-string round-trip byte-exact for canonical strings over an eight-string corpus, and every malformed input asserted on the **exact** documented message and variant (no substring assertions, assurance §4, TS-13 green with no allowlist entry added); the fixture pipeline's fused group boundaries pinned in `const` items; the mandatory-never-fuses canary proven over five arrangements rather than one. No bench was added, so TS-10 has nothing to record. **Deps:** P2-1. **Non-goals:** observer hooks (P2-3); running real passes (P2-9); optimization tiers scaling budgets rather than pass sets (P3-10); a JS/WASM plugin pass tier (phase 6).

## P2-3 — `PassObserver`

**Landed 2026-08-19** — full record: [phase-2-records/p2-3.md](./phase-2-records/p2-3.md).

**Deliverable:** the seven-hook observer plus the four in-tree observers, with fusion groups reported explicitly so timing never lies.

**Steps:**

- [x] Seven hooks: `before_pipeline` / `after_pipeline`, `before_pass` / `after_pass`, `before_analysis` / `after_analysis`, `on_fail` _(all seven with empty default bodies; `on_fail` suppresses `after_pipeline`, since a run that failed did not finish)_
- [x] Timing observer emitting the **P0-11** profile-export schema through `vize_carton::profiler`, reusing `SpanAttribution` rather than a second attribution model _(one attributed span per **walk**, keyed on the group's lead pass)_
- [x] Folio-printing observer, budget-counting observer (the walk counter P2-12b reads), remark sink (a no-op sink until P3-13) _(the folio observer binds to the **existing** P0-10 `Folio` trait rather than waiting for P2-4's derive — recorded as a deviation, since the contract expected it to consume P2-4)_
- [x] **Fused groups are reported as one walk with their member passes named** _(`PassEvent` carries its `FusionGroup`, `is_group_entry`/`is_group_exit` and `group_members`; the law is pinned by `tests/pass_observer_law.rs` and again at the timing level by `tests/pass_observer_timing.rs`)_
- [x] Attachment is checked once per pipeline, never per node (guardrail 1); the no-observer path must compile to the un-observed pipeline _(dispatch is **static**, so there is no attachment check at all — strictly stronger than once per pipeline; no `dyn PassObserver` is offered, on purpose)_

**Acceptance:** zero cost when nothing is attached, pinned by the bench pair `davinci_pipeline_unobserved` / `davinci_pipeline_no_observer` — both measured at **`allocs = 0`**, alloc-identical, both registered in `budgets.toml [bench]` and reported `alloc-gated … ok` by `tools/commands/davinci/bench-compare.rs` (TS-10). The fusion-group reporting law is pinned by ordinary integration tests under `crates/vize_davinci/tests/`, so they run in the default `cargo test --workspace` lane (the P1-5/P1-7 counter-law shape). Timing output reaches the P0-11 export with the right key and attribution, asserted directly; schema conformance stays pinned where the strict validator lives (TS-15, `vize_carton`) rather than copied — see the record. TS-1 (61 → 71 tests), TS-13 green with no allowlist entry added. **Deps:** P2-2. **Non-goals:** remark _content_ (P3-13); the Spolvero transport (P2-19) and UI (P2-18); provenance materialization policy (P2-8 records the pairs, the ring-buffer-vs-full decision is P2-12b's fusion measurement).

## P2-4 — Folio derive + `davinci-opt` pipelines

**Landed 2026-08-20** — full record: [phase-2-records/p2-4.md](./phase-2-records/p2-4.md).

**Deliverable:** `#[derive(Folio)]` and a `davinci-opt` that runs pipelines, so every later stage gets its dump and its pass tests for free.

**Steps:**

- [x] New proc-macro crate (`crates/vize_davinci_derive/`, workspace member, `experimental`, `publish = false`). It is a **host build dependency and stays `std`** — record that edge for P2-14's audit _(recorded; `cargo tree -i vize_davinci_derive --workspace --edges normal` shows exactly `vize_davinci`, and the generated code names only `::core` and `vize_davinci` support paths, so it compiles inside the `no_std` crate and the wasm32-wasip2 lane)_
- [x] The derive generates the existing trait's exact shape (`crates/vize_davinci/src/folio.rs:81`): `print(&self, w, mode: FolioMode)` and `parse(input) -> Result<Self, FolioError>` with 1-based line numbers, stable field order from the type shape, and the normalization rules already written in [`folio-format.md`](./folio-format.md) (sorted map iteration, stable sequential ids, fixed section order, empty sections omitted, LF) _(the generated grammar is written down in that doc's new "Derived pages" section; stable sequential ids are recorded there as **not applicable** — renumbering requires knowing which field is an id, which is semantic)_
- [x] **Derive the mechanical trio only** (print / parse / field order) — the ODS lesson. Anything carrying a semantic decision (what `Display` elides, what a section means) is hand-written and reviewed _(a derived page prints the same canonical text in both modes; `CroquisFolio` keeps its hand impl — the record measures why it cannot move)_
- [x] Extend `crates/vize_davinci/src/bin/davinci-opt.rs`: add `--pipeline "<syntax>"` and generalize `--stage <s>` beyond `croquis`. Today `--roundtrip <file>` is a **required** flag (`davinci-opt.rs:53`); it becomes one of two alternatives, and the existing usage string, exit codes (0 identity / 1 mismatch / 2 usage) and `--stage croquis` behavior stay byte-identical _(roundtrip output and exit codes byte-identical, pinned by the 14-fixture regression test; the usage string gained a **second line** for the new alternative and two messages had to change with the flag contract — each is itemized in the record's deviations)_
- [x] insta pass-test harness: folio in → pipeline → **full normalized folio** snapshot out, with targeted structural asserts only as supplements (assurance §4); `assert_folio_snapshot!` (`folio.rs:114`) is the printer _(`tests/pipeline_snapshot.rs`; no pass bodies exist until P2-9, so the plan runs no-ops and the supplements pin the walk accounting through the budget observer's own derived page)_

**Acceptance:** TS-16 per derived type — `print(parse(t)) == t` byte-exact in `Full` mode and `parse(print(v)) == v` structurally, with `Display` explicitly carrying no round-trip law. TS-17 established: at least one pass test per landed pass. The 14 committed P0-10 croquis fixtures still round-trip byte-identically through the extended binary (regression pin). TS-1, TS-13. **Deps:** P2-2, P2-3. **Non-goals:** deriving `Display` prose or semantic equality; a folio _diff_ format (P2-13 prints per-pass folios; diffing is C-3); the independent Lean folio checker (C-23).

## P2-5a — `vize_s2` S2 op and type family

**Landed 2026-08-20** — full record: [phase-2-records/p2-5a.md](./phase-2-records/p2-5a.md).

**Deliverable:** the S2 crate and its ops — the pivot stage and the primary consumer surface. At planning time, the S2 implementation crate did not exist in the tree.

**Steps:**

- [x] `crates/vize_s2/` created and added to `[workspace] members` in the root `Cargo.toml`; `publish = false`, `metadata.vize.stability = "experimental"`, `#![no_std]` plus `extern crate alloc;` from birth _(and TS-11 empty is mechanical: `cargo tree -i vize_s2 --workspace` prints only the crate itself)_
- [x] Op enums: element / component / text / interpolation / `ui.if { regions }` / `ui.for { binding, region }` / `ui.slot` / `ui.model { contract }` / `vue.directive`. **Regions are owned by their op** — this is what makes fusion tractable, because the enter/exit sibling mutation in `crates/vize_atelier_core/src/lane/structural.rs` (which merges `v-else` branches on the parent's child list) is precisely the re-visit source a region-owning `ui.if` never needs _(two enums: `Op` for region positions, `BindingOp` for the attached `ui.model`/`vue.directive` — the type system rules out a floating binding instead of a verifier rule)_
- [x] `ui.model` carries the **binding contract only** (what is read, what is written, the value-type flow), with element kind and dialect modifiers as attributes. Realization is never expanded in S2; IME/composition handling is runtime-owned by declaration (architecture, charter #23 tiering) _(`BindingContract { read, write }` with the value-type flow as the pair's documented shared-type law — the record states why flow data today would be a one-variant enum or speculation)_
- [x] Whatever is genuinely Vue-specific stays a `vue.*` dialect op instead of shaping the core — the fairness litmus test P2-16 then exercises _(exactly one dialect op, `vue.directive`; `ui.bind`/`ui.on` deliberately absent until the transform that needs them, P2-9)_
- [x] **Drop-free by construction**: every type arena-resident through `vize_carton::{Box, Vec}`, whose `needs_drop` const assertion is the enforcement (P1-10 measured it catching two real violations); no `impl Drop` anywhere in the crate _(grep 0; `!needs_drop` const asserts restate the property on `Op`/`BindingOp`/`Region` directly)_
- [x] Node-size `const` asserts per op type, guarded by `#[cfg(target_pointer_width = "64")]` (P2-14 makes wasm32 required) _(all fifteen types pinned; the figures are expected to move when P2-5b replaces `ExprSlot`)_
- [x] Exhaustive-match canary: a test that matches every variant with no `_` arm, so adding a variant breaks it. No `_` arms anywhere downstream _(proved by injection twice — the crate's own matches break, and with those patched the canary test alone still breaks; record § "The canary, proved by injection")_
- [x] S2 folio page from birth via the P2-4 derive _(hand-written owned `DisegnoFolio` instead — the P2-4 boundary applied and recorded: the derived grammar is flat by construction, the S2 artifact is region-nested by its central design decision; grammar in [`folio-format.md`](./folio-format.md) "Disegno page")_

**Acceptance:** TS-16 on the S2 folio (`Full` byte-exact, `Display` no law); TS-1; the guarded size asserts compile; the exhaustive-match canary is _demonstrably_ broken by an injected variant and green after handling it (the P0-7 staleness-check pattern — prove the canary, do not assume it); `grep -rn "impl Drop" crates/vize_s2/src` → 0; `cargo build -p vize_s2 --target wasm32-wasip2` green; TS-11 empty (nothing consumes S2 yet); TS-13. **Deps:** P2-1, P2-4. **Non-goals:** the expression reference (P2-5b); lowering into it (P2-8); the verifier (P2-6); speculative `vue.*` ops — a dialect op lands with the transform that needs it (P2-9); S3 (`vize_impeto`, phase 3).

## P2-5b — `ExprRef` contract, including the retained-`None` classes

**Landed 2026-08-20** — full record: [phase-2-records/p2-5b.md](./phase-2-records/p2-5b.md).

**Deliverable:** `ExprRef<'a>` with a written, tested, folio-serializable policy for **every** expression class the parser actually produces.

**Split reason** (recorded per the plan README): the provisional block assumed `ExprRef { Js, Foreign }` was total. P1-5 measured that the retained AST exists only for text that parses as one complete TS expression covering the whole content, and P1-9 measured 11.73% of corpus rewrites landing outside it. The `None` classes are a real design question the provisional text did not know about, and they are large enough to deserve their own review.

**Steps:**

- [x] `ExprRef<'a>` with the two architecture variants — `Js(&'a oxc_ast::ast::Expression<'a>)` and `Foreign(&'a ForeignExpr<'a>)`; `Foreign` is **type only** until phase 6 (charter #28), carrying dialect id + source slice + span + side tables _(one recorded deviation: `Js` points at a `JsExpr` carrying the AST reference **beside** its exact text and authored span, because this task's own folio clause needs slice + span at print time and a standalone-parsed expression's oxc spans are text-relative)_
- [x] **Decide the `None` classes and record the decision.** The measured shapes are: v-for values (`item of items` — the splitter synthesizes sub-expressions that never existed as template text, and JS `in` associates left while Vue's v-for grammar splits at the first viable `in`/`of`, so they genuinely disagree on `a in b in c`), v-on multi-statement bodies, nesting-guard-refused text (`vize_carton::expression_guard::expression_is_safe_to_parse`, refused _before_ parsing and before counting), text oxc rejects, and compound expressions rebuilt from source slices. Candidate resolutions to weigh: (a) a third variant carrying slice + span + a classified reason, with **pessimal documented semantics from day one** (the LLVM `undef`/`poison` regret, imported as a rule in `prior-art.md`); (b) widen the retained contract in `crates/vize_armature/src/parser/expression.rs` so the classes shrink; (c) both. Record which, and the measurement that picked it _(**(a)**, with (b) deferred to P2-9's measurement — two classes can never carry an AST so the variant is forced regardless, and the pessimal laws make a later widening monotone-safe; the five classes are a closed `OpaqueReason` enum with the text-classifiable/position-classified split documented; measurement in the record, run twice, identical)_
- [x] **Owned folio payload**, because arena references cannot persist across a compile (P1-11's contract, enforced by `'static` assertions and the debug arena-generation stamp in `crates/vize_carton/src/allocator/generation.rs`): `Js` serializes as source slice + span and re-parses into the arena on load; `Foreign` as dialect id + source + span; the escape variant as reason + slice + span _(`js(…)`/`foreign(…)`/`opaque(…)` tokens, grammar in `folio-format.md`; the load is total — refused text loads as `Opaque`, never a panic)_
- [x] Arena-reset replay test: print a folio → drop the `pool::acquire()` guard (arena reset) → parse → structural equality. This is P1-11's resident-cache reset scenario applied to folios _(`crates/vize_s2/tests/expr_replay.rs`, including the reload into a second pooled arena)_
- [x] The capability contract (enumerate referenced bindings, classify const-ness, map spans, emit for a target) is resolved **per file, never dyn-dispatched per node** (guardrail 1) _(`ExprDialect`, generic per-file binding, no `dyn` offered; `enumerate_bindings` names the #4365 seam without adding another scanner)_

**Acceptance:** TS-16 including `Js`, `Foreign` and escape-variant `Full`-mode fixtures **and** the arena-reset replay test; size asserts; TS-1; TS-11 empty. The class sizes backing the decision are machine-measured, not asserted: rerun the P1-7/P1-9 counters and record the per-class numbers in the PR —

```sh
VIZE_DAVINCI_DIFFERENTIAL_CORPUS=tests/_fixtures/_git \
  cargo test -p vize_atelier_sfc --features davinci-differential \
  --test davinci_differential -- --nocapture
```

**Review point:** the maintainer judges the escape variant's semantics against the prior-art rule — an escape variant without pessimal documented semantics is the failure this milestone exists to prevent. **Deps:** P2-5a. **Non-goals:** implementing a MoonBit dialect (phase 6, charter #28); resolving P1-8's scanner waiver ([#4365](https://github.com/ubugeeei-prod/vize/issues/4365)) — this task names where the single implementation would live and encodes neither resolution; deleting `steps/expression/reparse.rs` (P2-9 measures whether the class shrinks; deletion needs the wider contract to land first).

## P2-6 — S2 verifier v1

**Landed 2026-08-20** — full record: [phase-2-records/p2-6.md](./phase-2-records/p2-6.md).

**Deliverable:** the between-pass verifier, debug/CI only, with an invalid-folio fixture set.

**Steps:**

- [x] **Local checks only** (GHC Lint discipline): region nesting, id resolution (every `NodeId` a side table references resolves), expr-ref liveness, canonical-form invariants per `PassKind` _(six-code catalogue S2V001–S2V006 in one page-order walk; rigor escalates at the first `MandatoryLowering` pass and only there)_
- [x] Expr-ref liveness reuses the mechanism already in tree rather than inventing one: the debug arena-generation stamp (`Allocator::stamp` / `assert_stamp_current`, `crates/vize_carton/src/allocator/generation.rs`) panics on a value read against a reset arena _(`VerifyObserver::check_live` delegates to `assert_stamp_current`; one artifact-level stamp today because `ExprSlot` is zero-sized — the method is the recorded P2-5b seam)_
- [x] Each invariant documented in [`folio-format.md`](./folio-format.md) — the format doc is where "canonical" is written down _("S2 verifier invariants" section: codes, rigor, node numbering, the liveness seam, the fixture contract)_
- [x] Runs between passes in debug/CI **through the P2-3 observer**, never in the release hot path (guardrail 5: verification never ships) _(`VerifyObserver` follows the `FolioObserver` precedent — hooks track rigor, artifact-holding callers invoke the checks; release shape is a ZST with empty bodies, const-asserted)_
- [x] Invalid-folio fixture set: hand-built invalid artifacts, each committed with its exact expected diagnostic (code + span + full message, canonical `en` locale) _(15 pages in `crates/vize_s2/tests/fixtures/invalid/`, each with a `.expected` twin compared whole-file; the two lanes page text cannot encode — id resolution, liveness — are pinned with exact oracles in `tests/verifier_observer.rs`)_

**Acceptance:** TS-18 established — the verifier rejects **every** committed invalid artifact with the exact diagnostic, no partial matching (TS-13 enforces that mechanically); a release build makes zero verifier calls, asserted by the `cfg` shape plus the P2-3 zero-cost bench (TS-10); TS-1. **Deps:** P2-5a, P2-3. **Non-goals:** whole-program or fixpoint checks — those are barrier analyses and the S3 equivalent is P3-1's phase validator; the independent Lean folio checker (C-23); verifying S1 (P2-7's `render == source` is its own verifier).

## P2-7 — S1 Vue surface tree

**Landed 2026-08-20** — full record: [phase-2-records/p2-7.md](./phase-2-records/p2-7.md).

**Deliverable:** the lossless Vue-template surface tree with typed holes.

**Steps:**

- [x] Lossless tree with trivia; `Unexpected` / `Missing` typed structural error nodes (SwiftSyntax import), so every consumer sees one uniformly-shaped tree with holes and S1→S2 has a **single documented hole policy** instead of per-consumer error special-casing _(three clauses at the crate root: `Missing` tokens plus the node-level `ElementClose::Missing`; `Unexpected` children; verbatim intra-tag `leading` — the third is the recorded SwiftSyntax deviation)_
- [x] `render(tree) == source` debug verifier asserted on **every** construction — the cheapest high-yield verifier in the prior-art survey _(`check_fidelity`, allocation-free incremental compare, `debug_assert!`ed in `parse`)_
- [x] Emitted by `vize_armature`, or as a thin layer over relief until relief splits; the relief split itself is not this task's (record which shape landed and why) _(a third shape landed: new crate `vize_s1` driving armature's public tokenizer `Callbacks` — the record has why neither offered shape survived the publish gate / lossy-relief measurements)_
- [x] Strings stay `&'a str` per P1-10 — trivia is a source slice or an arena copy, never an owned string; the tree is `Drop`-free and arena-resident, with the container const assertion as enforcement _(every string a source slice; `vize_carton::{Box, Vec}` throughout)_
- [x] Node-size `const` asserts, `#[cfg(target_pointer_width = "64")]`-guarded _(eleven figures pinned in `surface.rs`, all pointer-bearing so all guarded)_

**Acceptance:** TS-19 established — `render(parse(src)) == src` **bytes** as a property over the corpus and over the malformed-fixture set, including the `Unexpected` / `Missing` paths; TS-11 empty (S1 is additive here, nothing consumes it yet); guarded size asserts; TS-1, TS-13. **Deps:** P2-1. **Non-goals:** pug as an S1 dialect (charter #12, phase 4); the OXC-backed lossless **script/JSX** wrapper — "S1" reads wider than this task and the script side is phase 4's, with the formatter and autofix consumers; retiring the `vize_glyph` byte scanner (phase 4, charter #41); `vize_musea`'s hand parser (phase 4).

## P2-8 — S1→S2 Vue lowering

**Landed 2026-08-21** — full record: [phase-2-records/p2-8.md](./phase-2-records/p2-8.md).

**Deliverable:** a total lowering function, with provenance and a fuzz lane.

**Steps:**

- [x] **Total function, no rollback** (MLIR import): every input yields S2 or a diagnostic, never a panic and never a partial-then-abandoned state _(new conversion crate `crates/vize_s1_to_s2/` — the home decision and its dependency-direction reasoning are in the record; anything the existing op family cannot carry is an `Info` deferral or an `Error` plus a kept fragment, and the property is asserted over the whole S1 battery **and every prefix/suffix truncation of it** with the folio round-trip and the S2 verifier at `Canonical` rigor as the oracle)_
- [x] Hygiene scope-tags on synthesized identifiers (slot props, v-for scopes) so a later pass can never confuse a synthesized name with an author's _(`vize_s2::scope`: `ScopeTag` per introduction site, `ScopeOrigin::{Authored, Synthesized}` per binding, in a `SideTable<ScopeFacts>` keyed by page-order `NodeId`; P2-8 itself synthesizes no names — the record states why — and pattern-name enumeration stays at the one #4365 seam)_
- [x] `MacroExpansionInfo`-style provenance pairs recorded at each lowering decision (before/after, by pass name); provenance **survives failure** — partial S2 fragments are kept on error, Lean-InfoTree style, so the LSP and Spolvero stay live on broken SFCs _(`vize_s2::provenance::ProvenanceRecord`, fully materialized — the ring-buffer decision stays P2-12b's; a decision that produced nothing is still a record, pinned by test)_
- [x] v-for consumes P2-5b's decision for its alias/source sub-expressions rather than re-deriving it; the `a in b in c` disagreement recorded at P1-6 is the reason the retained AST of the v-for value must not be consumed naively _(the split is the shipped splitter's text grammar over the **untrimmed** value, then each sub-slice goes through the one shared admission rule; `OpaqueReason::ForValue` lands on the unsplittable whole and on the absent-alias position, both pinned)_
- [x] Fuzz targets for S1→S2 and the folio parsers added under `tests/fuzz/fuzz_targets/` (joining the five that exist: `css_parse`, `js_ts_expression`, `sfc_parse`, `template_compile`, `template_lexer`) and the new crate paths added to `.github/workflows/fuzz.yml`'s PR path filter _(`s1_lowering` and `folio_parse` — both assert the id-accounting and round-trip laws, not just no-crash; matrix entries, corpus seeding and the four crate paths added; cargo-fuzz cannot run on this machine, so locally the targets are `cargo check`-green in the isolated fuzz workspace and the record splits what ran here from what CI covers)_

**Acceptance:** TS-20 established — no panic on arbitrary bytes, diagnostics rather than crashes, with fixed crashes carrying deterministic reproducers (TS-8's convention). Every corpus template lowers or produces a diagnostic, asserted by a corpus-runnable entry in the P1-6/P1-7 differential-lane shape (`#[cfg(any(test, feature = "davinci-differential"))]`, env-var corpus widening, exact-pinned counts in the plain suite so a cfg regression fails loudly). TS-19 unaffected; TS-11 empty; TS-1, TS-13. **Deps:** P2-5b, P2-7. **Non-goals:** JSX lowering (P2-16); pug (phase 4); replacing the atelier transform lane (P2-9); emitting anything (P2-11).

## P2-9 — Core transforms as S2 passes

**Deliverable:** the core transform lane re-expressed as classified S2 passes, with the old lane still live. **Explicitly marked small series** (the plan README's permitted exception): one reviewable PR per transform, all under this ID.

**Steps** — the checklist is the actual directory, `crates/vize_atelier_core/src/steps/`, reached through the Rust module `vize_atelier_core::steps` by ordinary module discovery. Historical records predating the module-layout migration may call this directory `src/transforms/`:

- [x] `v_if.rs` → `ui.if` regions — the first and highest-value port, because regions replace the sibling mutation in `src/lane/structural.rs` _(installment 1, 2026-08-21: the structural half was already the P2-8 lowering's; the pass body is the branch-key lift + `VIfSameKey`, classified `MandatoryLowering`/barrier in `vize_s1_to_s2::pass::vif` — see the [series record](./phase-2-records/p2-9.md))_
- [x] `v_for.rs` → `ui.for` region _(installment 2, 2026-08-21: the step file ports **whole** into the P2-8 lowering — split, both grammar diagnostics, the region restructure, the scope recording; the pass body is the hygiene consumption (validate + publish `ForFacts`), classified `MandatoryLowering`/barrier in `vize_s1_to_s2::pass::vfor` with the preserving-mandatory taxonomy tension recorded — see the [series record](./phase-2-records/p2-9.md))_
- [x] `v_slot.rs` (+ `v_slot/params.rs`, `v_slot/validate.rs`) → slot normalization _(installment 3, 2026-08-21: the spelling lowers to the new `ui.slot-content` binding op (name/modifiers/params as authored, scope facts re-keyed to the binding op); the pass body is the canonical grouping published as `SlotFacts`, the four `validate.rs` diagnostics, and the series' first `ScopeOrigin::Synthesized` producer (bare-`v-slot` default name, implicit default group), classified `MandatoryLowering`/barrier in `vize_s1_to_s2::pass::vslot` — see the [series record](./phase-2-records/p2-9.md))_
- [x] `text.rs` → text / interpolation merging _(installment 4, 2026-08-21: the step file is **dead in the shipped lane** — its two behaviours ship from parse time (armature's condense) and codegen time (`codegen/children.rs`'s run grouping) — and both absorb into the P2-8 lowering (`vize_s1_to_s2::lower::text`), forced by comment visibility and by P2-5b's rule that `OpaqueReason::Compound` is assignable only by the S1→S2 lowering; a mixed run lowers to one `ui.interpolation` carrying the series' first `Compound` producer with the parts recorded beside the tree, and the pass body is the compound-law consumption (validate + publish `TextFacts`), classified `MandatoryLowering`/barrier in `vize_s1_to_s2::pass::text` — see the [series record](./phase-2-records/p2-9.md))_
- [x] `hoist_static.rs` (+ `hoist_static/props.rs`, `static_type.rs`) → an S2 **analysis** pass (a fact, not a mutation) _(installment 6, 2026-08-21: the step file is **live in the shipped lane** (`lane.rs:295`, `codegen/children.rs`) and splits three ways — the analysis (the `StaticType` lattice + the prop/nested/native predicates) is the pass body `vize_s1_to_s2::pass::hoist`, published per owner as `StaticFacts`; the position/option-dependent decisions and the realization stay with P2-11 — classified the series' **first `Optional`/`Fusable`** pass (`Preserved::ALL`): the fusion machinery forms its first non-barrier singleton group (`group_count()` 6, pinned), the const-classification consumes P2-5b's pessimal law for real (an opaque bind value is never constant), and the differential lane compares the hoist-armed shipped run's actual decisions against the facts' predictions — see the [series record](./phase-2-records/p2-9.md))_
- [x] `element.rs`, `v_bind.rs`, `v_on.rs`, `v_model.rs`, `v_memo.rs`, `v_once.rs` → the normalized-binding ops; `v_model.rs` lowers to `ui.model`'s **contract**, not its realization _(installment 5, 2026-08-21: five of the six step files measured **dead in the shipped lane** — the living code is `lane/element.rs` (model expansion + validation) and codegen; `ui.bind`/`ui.on` land in `vize_s2` (canary proven broken-then-fixed) and lower with the parser's same-name and dot-`prop` shorthands mirrored; the outlet gains its props surface, `<template v-if>` wrapper keys ride a lowering capture channel, the v-if pass gains the dynamic/wrapper/outlet key arms, and the one new pass is `vize_s1_to_s2::pass::vmodel` — the live lane's two model validations, the series' **first `MandatoryDiagnostic`**; `v-once`/`v-memo` measured codegen-only, ops land with the stage that reads them (P2-11) — see the [series record](./phase-2-records/p2-9.md))_
- [x] `legacy.rs` / `legacy_filters.rs` → `vue.*` dialect ops _(installment 7, 2026-08-23: three stacked PRs — ops `#4633`, lowering `#4634`, `legacy-sugar` pass `#4637`. No ricalco `_legacy` cargo feature: the crate is `publish = false`, Vue 3 is one `needs_sugar()` false on `LegacyCaps::VUE3` selecting the existing six-pass table (`walks=6`); Vue 2 prepends a `MandatoryLowering`/barrier (`Preserved::NONE`, `walks=7`). Mixed text-runs that absorbed a pipe into a compound opaque are not `ExprRef::Filter` and are not wrapped — recorded. See the [series record](./phase-2-records/p2-9.md))_
- [x] `steps/expression.rs` and its 13-file subtree stay on the old lane in this task — it is P1-7/P1-9's working set and P2-5b owns its future _(installment 11, 2026-08-30: the hydrated corpus re-measure compiled 41,580 files and showed the residual class still at **11.73%**; this records the boundary instead of porting the subtree)_
- [x] Every ported pass is classified (`MandatoryDiagnostic` / `MandatoryLowering` / `Optional`) and marked fusable or barrier — **review point**, since a misclassified mandatory pass silently leaves the fusion budget _(installments 1–7: five `MandatoryLowering` barriers, one `MandatoryDiagnostic` barrier, one `Optional`/`Fusable`, plus Vue 2's prepended `legacy-sugar` barrier; Vue 3 `group_count()==6` unchanged)_
- [x] The old lane stays live behind the in-phase flag (charter #26) and is deleted at the exit gate _(installment 11 records this as the accepted P2-9 boundary; deleting the old transform lane remains an exit-gate action)_
- [x] **Differential lane, the P1-6/P1-7/P1-9 shape**: `#[cfg(any(test, feature = "davinci-differential"))]` dual-run comparator inside the migrated path, compared at the DOM-output level; process-global counters; a plain-suite coverage witness pinning exact counts so a cfg regression that disarms the lane fails; a corpus-runnable entry with its exact command recorded. Divergence panics — investigate, never average _(installment 11 reran the env-var corpus witness over the canonical hydrated 146-submodule fixture tree: 41,580 files compiled, zero divergence, scope proof printed)_
- [x] Measure and record the effect on the P1-9 residual classes: does region-structured lowering shrink `steps/expression/reparse.rs`'s 11.73%? A number from the existing `retained::differential` counters, not a prediction _(measured in installment 11: admitted 801,305, legacy_total 106,532, `admitted_pct=88.27`, residual **11.73%**)_

**Acceptance:** per-pass full normalized folio snapshots (TS-17); DOM output through the **old** codegen unchanged — `rust-script tools/commands/davinci/corpus-diff.rs --surface compiler` empty with scope proof (TS-11); differential lane green over the corpus, zero divergence (TS-25); the touched benches' `allocs` re-recorded in `budgets.toml` as tightened numbers with their measurement (TS-10, ratchet); TS-1, TS-13. **Deps:** P2-6, P2-8, P2-12a. **Non-goals:** the DOM backend itself (P2-11); the SSR and Vapor lanes (phase 3 — they stay on the old lane, which is the strangler design, not an oversight); deleting the old transform lane (exit gate); porting `steps/expression/` (P2-5b decides its contract first).

**Series log** — one line per landed installment; the full account is the [series record](./phase-2-records/p2-9.md):

- **1 (2026-08-21):** the series substrate — pass bodies in `vize_s1_to_s2::pass`, comparator in `vize_atelier_core` dev-dep test space (the publish gate's stripped-dev-dep carve-out), lane flag `VIZE_DAVINCI_TRANSFORM` — plus the `v-if` port: TS-17 snapshots committed, differential lane zero-divergence over the 18-template battery (pinned) and 12,017 of 12,021 corpus templates (4 hard-parse-error skips, named), the 11.73%-class seed re-measured twice (12.73% on today's hydration state, byte-identical to P2-5b's runs — the series baseline).
- **2 (2026-08-21):** the `v-for` port: old `v_for.rs` measured **fully absorbed** by the P2-8 lowering; the pass body is the hygiene consumption of the P2-8 `ScopeFacts` (validate against the binding surface, publish the consumed `ForFacts` view, synthesize nothing), `MandatoryLowering`/barrier with the fusion plan re-pinned at two lone barriers; comparator gains the for projection (`renderList`'s input surface): battery 30 templates exact-pinned, corpus ×2 identical — 12,017 compared, zero divergence, 2,882 for-ops (2,882 values / 1,083 keys / 3 indexes); the residual class re-measured twice, **12.73% unmoved** (no rewriter feed yet — `ForFacts` is the prepared feed).
- **3 (2026-08-21):** the `v-slot` port: the spelling lowers to the new `ui.slot-content` binding op (authored surface kept; scope facts re-keyed to the binding op, ending the shared-carrier gap), and the pass body is the canonical grouping (`SlotFacts` per component), the four `validate.rs` diagnostics relief-exact, and the series' **first `ScopeOrigin::Synthesized` producer** (bare-`v-slot` default name + implicit default group, authored-vs-synthesized pinned both directions), `MandatoryLowering`/barrier — three lone barriers, `walks=3`; comparator gains the lane-neutral slot projection (units by authored-tag nativeness, groups with the invented class as the cross-lane Synthesized witness, outlet names): battery 44 templates exact-pinned (predicted-then-confirmed), corpus ×2 identical — 12,017 compared, zero divergence, **6,090 units / 10,498 groups (2,993 invented) / 2,897 outlets**, counted classes 111 conditional-carrier + 5 comment-filler; the residual class re-measured twice, **12.73% unmoved** (`SlotFacts` joins the prepared feed).
- **5 (2026-08-21):** the element/binding family: `ui.bind`/`ui.on` land (the op-family canary proven again; the outlet gains attributes + bindings; the parser's same-name and dot-shorthand normalizations mirrored byte-for-byte), `defer.v-bind`/`defer.v-on`/`defer.slot-props` retire (the lowering corpus lane's with_diagnostics drops 10,142 → 804), the v-if pass gains the dynamic-key, wrapper-key and outlet-key arms (installments 1–2's counted skips now compared: corpus 45 static + 81 dynamic keys), the misplaced-on-outlet diagnostic lands, and the one new pass body is `v-model`'s two live-lane validations — the series' **first `MandatoryDiagnostic`**, five lone barriers, `walks=5` (no fusable pass: the measured answer to the review point); comparator gains the binding-surface projection (owners × attrs/binds/ons/directives/reconstructed model contracts): battery 75 templates exact-pinned, corpus ×2 identical — 12,017 compared, zero divergence, **120,734 owners / 119,584 attrs / 54,698 binds / 12,877 handlers / 1,260 directives / 7,627 model contracts**, counted classes incl. the new in-table tree-construction deviation (103 templates); the residual class re-measured twice, **12.73% unmoved** (structural: no shipped `rewrite_expression` feed exists for any series installment to move — the ops are the prepared feed).
- **6 (2026-08-21):** the `hoist_static` port — the series' **first `Optional`/`Fusable` pass** and its designated analysis pass: the step file measured **live in the shipped lane** (the first since installment 3 — `lane.rs:295` runs it, codegen reads `is_static_node`), split three ways — the `StaticType` lattice + prop/nested/native predicates become `vize_s1_to_s2::pass::hoist`'s per-owner `StaticFacts` (the `foreign` bit added by the corpus's own counter-example: a component inside `<svg>` inherits the namespace the `ns != Html` arm reads), the position/option-dependent decisions and the realization stay with P2-11; the fusion machinery measured on its first fusable pass: `group_count()` 5 → 6, the first **non-barrier singleton group** (const-pinned; `is_fully_serialized()` still literally true — fusion buys nothing until a fusable neighbour lands), `walks=6`; the const rule consumes P2-5b's pessimal law for real (opaque ⇒ never constant, battery-pinned) and is deliberately weaker than the shipped classifier on JS (no allowlist, no `this`, no TS spellings — one-sided by design, `consts_templates` 96 counted); the cacheHandlers analysis half measured **not this pass's fact** (options + binding metadata + ancestor context the realization walk carries — lands whole at P2-11, boundary recorded); comparator gains the decision projection (a hoist-armed second shipped run's actual mutations vs the facts' predictions, replay-controlled by the exported `get_static_type`, a shape pre-check turning nesting deviations into a counted class — measured **0** corpus-wide): battery 90 templates exact-pinned, corpus ×2 identical — 12,017 compared, zero divergence, **97,546 verdicts / 7,886 whole / 5,934 props agreed**, counted classes 513 comment elements + 344 builtin subtrees + 96/222/77 consts/classifier/models templates + 103 table; the residual class re-measured twice, **12.73% unmoved** (`StaticFacts` joins the prepared feed).
- **4 (2026-08-21):** the `transform_text` port: the step file measured **dead in the shipped lane** (zero call sites — condense ships at parse time, merging at codegen time), and both halves absorb into the P2-8 lowering (`lower::text`) because only S1 still sees comments (run boundaries and remove-vs-condense neighbours) and P2-5b assigns `Compound` production to the lowering alone; mixed runs lower to one `ui.interpolation` carrying the series' **first `OpaqueReason::Compound` producer** (pessimal laws from the first byte, parts recorded beside the tree), and the pass body is the compound-law consumption (tiling + rebuild laws, publish `TextFacts`), `MandatoryLowering`/barrier — four lone barriers, `walks=4`; comparator gains the text projection (`createTextVNode` units with static/dynamic parts): battery 58 templates exact-pinned, corpus ×2 identical — 12,017 compared, zero divergence, **37,575 units / 28,783 static + 11,588 dynamic parts / 2,135 compounds**, counted classes 88 entity-skipped + 0 v-pre + 6 rawtext-excluded; the metamorphic `merge_text`/`condense` normalizations **ratcheted out** (the P2-15 checklist consumed; its corpus canary caught two split-adjacency bugs in the first cut, fixed via text-group classification); the residual class re-measured twice, **12.73% unmoved** (`TextFacts` joins the prepared feed).
- **7 (2026-08-23):** Vue 2 dialect sugar, last transform-directory line: ops + admission + `legacy-sugar` pass (no ricalco `_legacy` feature — `needs_sugar()` on `LegacyCaps`); Vue 3 `walks=6` unchanged, Vue 2 `walks=7`; mixed text-run pipes stay compound-opaque (not wrapped); Vue 2 atelier comparator and P1-9 residual re-measure **not this installment** (the series box in `phase-2.md` stays open).
- **8 (2026-08-24):** Vue 2 atelier comparator: `compare` takes a dialect (Vue 3 path unchanged); new `legacy`-gated witness dual-runs six sugar templates exact-pinned; interpolation filters remain a named text-projection gap (authored `msg | cap` vs rewritten `_filter_cap(msg)`); P1-9 residual re-measure still not this installment (the series box in `phase-2.md` stays open).
- **9 (2026-08-24):** interpolation-filter text projection: measured lone `{{ msg | cap }}` (authored vs `_filter_cap(msg)`) as a wrong-stage read — legacy `process_expression` is off, S2 legalize wraps — and wrap-equals them (`parts_filter=2`); mixed `hello {{ msg | cap }}` already agreed on the authored pipe (compound-opaque, not wrapped) and is now in the battery so it is not silently skipped; Vue 2 witness 9 templates exact-pinned; P1-9 residual still unmeasured (the series box in `phase-2.md` stays open).
- **10 (2026-08-24):** P1-9 residual re-measure attempted: `cargo test -p vize_atelier_sfc --features davinci-differential --test davinci_differential -- --nocapture` green on battery+ladder (`transform_rewrite_comparisons=1408`); the corpus env-var sweep panicked `corpus sweep found no .vue files` because `tests/_fixtures/_git` is 146 unhydrated gitlinks. Residual percent **not invented**. Series box stays open.
- **11 (2026-08-30):** hydrated P1-9 residual re-measure: `VIZE_DAVINCI_DIFFERENTIAL_CORPUS=tests/_fixtures/_git cargo test -p vize_atelier_sfc --features davinci-differential --test davinci_differential -- --nocapture` green over the canonical 146-submodule corpus — 41,580 files compiled, zero divergence, transform rewrite split `admitted=801305 legacy_total=106532 admitted_pct=88.27`, residual **11.73%**. This completes P2-9's measurement obligation; `steps/expression/` and old-lane deletion stay outside P2-9 by contract.

## P2-10 — Style `v-bind()` ops

**Deliverable:** SFC style-block bindings visible as S2 ops (charter #13), so lint, the reactivity lattice and the type projection stop having a descriptor-level blind spot.

**Steps:**

- [x] Surface the existing css-vars coordination as S2 binding ops _(the five atelier_sfc sites stay the compile path; S2 ops land in ricalco so the published crate never depends on Davinci and css-var names stay frozen — [record](./phase-2-records/p2-10.md))_
- [x] The op carries the **CSS-block span**, not only the expression, so a diagnostic points into the style block; spans are file-absolute against the complete authored source, using the style-block content start as the offset base
- [x] The bound expression rides as an `ExprRef` under P2-5b's contract — CSS `v-bind()` contents are exactly the kind of text that may have no retained AST, so the class decision applies here first

**Acceptance:** the facts are visible in the S2 folio — a committed `v-bind()`-bearing SFC fixture whose folio pins them (TS-16, TS-17); **compile output unchanged**, `corpus-diff --surface compiler` empty (TS-11); TS-1. **Deps:** P2-9. **Non-goals:** lint rules or lattice consumers reading them (phase 4); a CSS S1 dialect (the `lightningcss` boundary is P2-14's audit, not this task); changing the emitted css-var naming (`scoped_v_bind_name` / `prod_scoped_v_bind_name` output is byte-frozen by TS-11).

## P2-11 — DOM backend on S2

**Deliverable:** `vize_atelier_dom` lowering S2 → codegen structure directly — the first strangler target, on the surface that holds the hard byte-parity bar.

**Landed 2026-09-06** — full record:
[phase-2-records/p2-11.md](./phase-2-records/p2-11.md).

**Current series evidence (2026-09-06):** 123 installments have landed through
[#5860](https://github.com/ubugeeei-prod/vize/pull/5860). Installments 84-123
open and close the production option surface the switch needed, since through
83 the lane only matched `compile_template`'s _defaults_ while production
compiles go through `compile_template_block`: `DomEmitOptions` carries module
mode, `prefix_identifiers`, non-inline binding metadata, `is_ts`, the SFC's own
`component_name` for self-references, `inline`, inline root prop hoists,
transform-time `_unref` helper order, inline template refs, merged
const-handler rules, non-simple cached prop layout, constant style and text-run
shortcuts, `cache_handlers`, printed-order cache slots, scoped CSS ids, runtime
names, option bundles, model/outlet families, the shared DOM battery, the
source-map-free production selector, S2 DOM section boundaries, in-tag option
routing, SFC namespace selection, ordinary comment output, source-map requests
handled around S2 with a verified compatibility map, experimental in-tag
comments, declarative custom-element patterns, bare static style merges and
disabled static-hoist routing, HTML re-entry close casing and the explicit
legacy selection guard, the audited DOM no-op `optimize_imports` selector,
static custom-element predicates, parser-recovered SFC self-closing sections
and the final DOM legacy lane flag deletion. The earlier S2 DOM lane also
covers the late directive and patch-site witnesses, residual component and
hoist-order witnesses, and the corpus-runnable plus CI DOM lanes. Real Project
Matrix run `33531193323` recorded canonical hydrated zero-divergence evidence
over 146 gitlinks, 142 ecosystem projects, 42,668 files and 42,279 compared
templates, and [#5860](https://github.com/ubugeeei-prod/vize/pull/5860) added
the live-source boundary test that keeps `VIZE_DAVINCI_DOM`, `DOM_LANE_FLAG`,
`dom_lane_selection` and `DomLaneSelection` out of the production DOM sources.

**Steps:**

- [x] `vize_atelier_dom` lowers S2 directly; the relief codegen-node universe (`NodeType` 13–20 codegen + 21–26 SSR codegen, of 27 variants total — `crates/vize_relief/src/relief/core.rs:10-42`) stops being **written** by the new path. It is still _read_ by SSR and Vapor until phase 3, so nothing is deleted here
- [x] In-phase flag `VIZE_DAVINCI_DOM=legacy` (charter #26), production-selectable while the phase is live, **named in the exit gate with its deletion**. P1-13's lesson governs: an undeleted old path is an unfinished deletion with an owner, not a permanent fallback
- [x] **Differential lane, the P1-9 shape**: dual-run old vs new DOM emission, compared byte-for-byte including helper usage, panicking on any difference; corpus command recorded in the task
- [x] **Waiver budget: zero.** DOM emitted output is the hard byte-parity bar (charter #23) and this is the most output-visible surface in the phase; any corpus diff is a bug in this task, exactly as P1-9 ran it
- [x] Patch-flag equivalence fixtures (the flags the new path computes must equal the old path's, per node, exactly)

**Acceptance:** `rust-script tools/commands/davinci/corpus-diff.rs --surface compiler --shards 2 --timeout-ms 600000` empty across the 144-project manifest with scope proof, run from clean fixtures (TS-11); differential lane zero divergence with its comparison count recorded as 144 DOM-output comparisons (TS-25); patch-flag equivalence fixtures exact (TS-1/TS-2); DOM bench `allocs` re-recorded in `budgets.toml` (TS-10); TS-13. **Deps:** P2-9. **Non-goals:** SSR and Vapor backends (phase 3); source maps from a structured S4 emitter (P3-9); deleting the relief codegen-node universe; the vapor run-then-discard double transform (P3-6).

## P2-12a — Phase-start baselines and pinned targets

**Landed 2026-08-19** — full record: [phase-2-records/p2-12a.md](./phase-2-records/p2-12a.md). One acceptance clause is carried rather than met (`corpus-coverage --check`); the record states why and where it goes.

**Deliverable:** the numbers phase 2 will be judged against, recorded **before** the work that could bias them.

**Split reason:** P1-13's gate could not tick "compile bench improvement ≥ target pinned at phase start" because neither a target nor a phase-start baseline ever existed, and it recorded that as a miss rather than inventing a number after the fact. Repeating that would make phase 2's exit unmeasurable too. Compounding it, the provisional P2-12 said "compare against the P0-3 walk baseline" — **P0-3 recorded expression re-parse counts, not walk counts**, and `budgets.toml [traversal]` is an empty reserved section. Pinning therefore becomes its own dependency-free phase-start task that must merge before P2-9.

**Steps:**

- [x] Record the **pre-S2 walk count** per ladder fixture per backend with a temporary counter hook on today's still-live pipeline — the exact P0-3 pattern (`vize_atelier_core::expr_parse_probe`, 18 sites, baseline committed to a plan doc). Ladder: `tools/benchmarks/crates/davinci_harness/fixtures/{small,medium,large,stress-deep,stress-wide,stress-interp}.vue` _(`crates/vize_atelier_core/src/walk_probe.rs`, 19 sites)_
- [x] Commit `davinci-road/plan/walk-baseline.md` with the counts, the exact reproducing command, and the two-run determinism proof (the P0-2/P0-5 convention)
- [x] Fill `budgets.toml [traversal]` (today: `# Populated by P2-12`) with the per-fixture ceilings. State the machine-independence reasoning explicitly, the way the `allocs` field docs were rewritten at P1-13: **walk counts, like alloc counts, are deterministic and machine-independent**, so `[traversal]` gates exactly from day one and does not wait for the Blacksmith reference runner _(18 entries, `<backend>_<fixture> = { walks, visits }`)_
- [x] Extend `tests/tooling/davinci-budgets.test.ts` to reconcile `[traversal]` against the probe ids **in both directions** — today it validates only the `[bench]` registry — so a fixture without a ceiling, or a ceiling without a fixture, fails _(landed as its own suite, `tests/tooling/davinci-traversal-budgets.test.ts`, so neither file passes the 350-line source budget; all three gates proven by injected failures)_
- [x] Pin the **phase-2 improvement target** in `budgets.toml`, in the quantities that are machine-independent (fused-compile `allocs` on the ladder and walk counts), with wall time explicitly report-only until the Blacksmith recording. Record the **phase-start rev** in the same table, so the phase-end re-bench has a defined "before" _(`[target.phase-2]`, phase-start rev `232870a8`)_
- [~] Corpus expansion audit for the surfaces phase 2 touches (charter #31, C-14): `rust-script tools/commands/davinci/corpus-coverage.rs --check` with its scope proof; any S2 construct with no real-project instance is recorded as "not represented — matrix fixtures only" _(**audit done** against the committed 142/142-hydrated report — `mathml` is the one S2 element kind with zero real-project instances; **`--check` itself is not evaluable** by CI or a normal working tree, which is a plan bug the record states and carries to the exit gate's C-14 line)_

**Acceptance:** `walk-baseline.md` committed and reproducible (two runs identical); `[traversal]` non-empty and reconciled by the extended budgets suites (TS-3); the target table present with non-zero values and the phase-start rev recorded; corpus-coverage `--check` green with scope proof (TS-12) — **the one clause not met, see the record**; TS-11 empty (the probe counts, it does not change behavior — the P0-3 precedent). **Review point:** the maintainer sets the target _numbers_; the artifact's existence and non-zero-ness is what CI checks, and the assurance doctrine forbids choosing them later to fit the result. **Deps:** none. **Must merge before P2-9.** **Non-goals:** the observer-based walk counter (P2-12b); recording the Blacksmith wall baselines and the CI bench lane (P0-4's open pending, not phase 2's to close); `[resource]` corpus-batch RSS, which still has nowhere to live until P5-11 and where P1-11's 766.5 MB → 171.1 MB figure is still stranded.

## P2-12b — Fused build path + walk-count instrumentation

**Deliverable:** `vize build` parsing straight to S2, with the traversal budget measured and gated.

**Steps:**

- [ ] Parse → S2 direct; **S1 is a capability**, materialized on demand only for consumers that need losslessness (formatter, lint autofix)
- [ ] Walk-count instrumentation through the P2-3 budget-counting observer, with fused groups reported as one walk (P2-3's law makes this honest)
- [ ] Gate against `budgets.toml [traversal]` / `walk-baseline.md`
- [ ] Answer the open question **"Fusion depth for the build path"**, which explicitly asks for a phase-2 prototype: measure walk count and whether fusing semantic-fact population into lowering costs diagnostic quality. Synthesized attributes fuse cleanly; anything needing lookahead (sibling `v-else`, slot collection) stays region-local. Record the answer in `open-questions.md`, converting the entry to a decided stub per that doc's own convention
- [ ] Decide provenance policy for the fused walk (off or ring-buffered in the CLI, fully materialized in resident/DevTool mode) with the measurement that chose it

**Acceptance:** TS-22 established — traversal count ≤ the `[traversal]` ceilings in `budgets.toml` on the fixture ladder, measured in CI and gated **exactly** (the alloc-gate reasoning, no tolerance); the walk law pinned by an ordinary integration test so it runs in the default `cargo test --workspace` lane; the fusion-depth open-questions entry updated with its measurement; fused-path benches' `allocs` recorded (TS-10); TS-11 empty for the fused path's output; TS-1, TS-13. **Deps:** P2-12a, P2-11, P2-3. **Non-goals:** optimization tiers scaling budgets (P3-10); the salsa resident tier and the snapshot tree (phase 5); making S1 unconditional; SSR/Vapor fusion (phase 3).

## P2-13 — Folio-after-change, `vize repro`, timing JSON

**Landed 2026-08-20** — full record: [phase-2-records/p2-13.md](./phase-2-records/p2-13.md).

**Deliverable:** the ICE policy made real (charter #30) plus the per-pass dump controls.

**Steps:**

- [x] `--folio-after-change` (hash-gated: print a pass's folio only when its hash changed) and `--folio-dir <path>`, on `davinci-opt` and the CLI compile path _(the mechanism is `FolioDump` in `vize_davinci` — IO-free, hashing the artifact's canonical `Full` text, which the Folio laws make interchangeable with the value. `davinci-opt --pipeline` dumps real pages; on `vize build` the driver has no folio-printable stage artifact until P2-12b, so the pinned behavior today is "directory created, zero pages" — asserted by test, so the flag is measured rather than decorative)_
- [x] Panic handler writes `repro.folio` — last-good stage dump + pipeline string + config — and the build reports **that file** as failed while other files continue, never silently degrading to possibly-wrong output _(the guard wraps the per-file compile: an injected panic is attributed exactly through the pass-manager driver; a real-compile panic is recorded with an empty `failed-pass` (a stated unknown, not a guessed pass). The last-good stage today is the authored source, `artifact-stage=source`. Live in unwind builds; the release profile's `panic = "abort"` stands — recorded, not decided here)_
- [x] `vize repro <file>` replays it. This is a **new** command: there is no `repro` module in `crates/vize/src/commands/` and no `Repro` variant in `crates/vize/src/cli.rs:19`'s enum, so the task adds both plus the module declaration in `crates/vize/src/commands.rs` _(added exactly so; the exact-equality comparison lives inside the tool — exit 0 = replayed to the byte-identical failure, 1 = diverged or completed, 2 = malformed)_
- [x] Timing JSON per the **P0-11** profile-export schema ([`profile-export.schema.json`](./profile-export.schema.json)) — the provisional text's "P0-4 schema" was a miscitation; P0-4 is `budgets.toml` _(`davinci-opt --timing-json <path>` writes `vize_carton::profiler`'s own export — one producer in the tree — with the P2-3 timing observer recording one span per walk; validated through the single TS-15 validator in the normal `davinci_test_support` dependency)_

**Acceptance:** TS-23 established — an injected panic produces a `repro.folio` and `vize repro` replays to the **same** failure, asserted by exact equality on the failure, not a substring; the file-scoped property asserted as an exact file set (a batch with one panicking file still emits every other file); the timing JSON validates against the schema (TS-15); TS-1, TS-13. **Deps:** P2-4, P2-3. **Non-goals:** `folio-reduce` (P3-14); the DevTool pass-timeline UI (C-3); crash telemetry or upload; auto-fallback on internal errors, which charter #26 forbids outright.

## P2-14 — `no_std` boundary audit + wasm32-wasip2 lanes

**Landed 2026-08-20** — full record: [phase-2-records/p2-14.md](./phase-2-records/p2-14.md).

**Deliverable:** the audit the open question calls for, then the CI lanes it licenses. **The audit comes first** — the workspace makes no `no_std` claim until it says so.

**Steps:**

- [x] Audit which dependencies genuinely support `no_std + alloc`: the oxc crates (which `vize_carton` and therefore everything downstream depend on), the map crate P2-1 picks, `lightningcss`, `compact_str` (which `vize_carton::String` aliases), `phf` (the interner's well-known table); and which are `std`-bound by nature — rayon (threads), salsa (resident-tier only), the CLI's filesystem and process layers _(ledger in [`no-std-boundary.md`](./no-std-boundary.md); measured corrections: none of the six oxc crates in the closure declare `no_std` at rev `fc702c1f`, salsa is not in `Cargo.lock` at all yet, and the P2-1 map is `FxHashMap` — a `std` type crossing the boundary by carton re-export)_
- [x] Document the approved boundary in a committed plan doc, including the P2-4 proc-macro crate as an approved `std` host-build edge _(committed as [`no-std-boundary.md`](./no-std-boundary.md) — **three** approved edges: the proc-macro, `vize_carton`, and the `davinci-opt` bin target the contract had not named)_
- [x] Separate the **core-compile lane** (`vize_davinci`, `vize_s2` only) from the full-CLI lane, which stays `std` _(the TS-24 step builds exactly the two claim crates; the full CLI stays `std` as the rest of the same job's clippy/test steps)_
- [x] Add the CI jobs to `.github/workflows/check.yml`: `cargo build -p vize_davinci -p vize_s2 --target wasm32-wasip2` and a `--no-default-features` build. **No `wasm32-wasip2` lane exists in any workflow today** — this task creates it, and it is required for the new crates only _(landed as **steps of `clippy-and-test`**, not new jobs — check.yml is over the 350-line ratchet, which forbids any net growth; required-ness is inherited from `test-report`'s `needs:` and pinned by `tests/tooling/davinci-portability-lane.test.ts`; the record § "The lane is a step extension" carries the arithmetic)_
- [x] Note that `vize_davinci` has no `[features]` section today, so `--no-default-features` is currently vacuous; the audit states what feature seam (if any) the crates should grow rather than leaving the flag decorative _(the audit's answer: **none yet** — a `std` feature would invert an unconditionally-`no_std` design; the vacuity is proven by the second lane build being a no-op rebuild, and the flag stays so the first real seam is gated from birth)_
- [x] Per P1-12's docs-truth precedent, the `no_std` claim must not appear in `docs/content/**` before the audit makes it true _(grep empty at landing; the audit's "Docs truth" section states the claim now tellable, for exactly the two crates and phrased as the boundary phrases it)_

**Acceptance:** TS-24 established and **required** for `vize_davinci` and `vize_s2`; audit doc committed; both lanes green; the guarded size asserts from P2-1/P2-5a/P2-7 prove their purpose by compiling on a 32-bit target. **Review point:** the maintainer approves the boundary — which dependency edges are accepted as `std`, and which crates the claim covers. **Deps:** P2-5a. **Non-goals:** converting existing crates to `no_std` (the audit says which _could_, it does not do it); the WASI component model as the out-of-process contract transport (charter #15, phase 6); wasm blob size budgets (charter #19).

The contracts for **P2-15 through P2-20** continue in
[phase-2-tasks-later.md](./phase-2-tasks-later.md), split under the 350-line
source budget.
