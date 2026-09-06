# P2-11 Installment 123 - DOM Legacy Lane Flag Deletion

> Part of the [P2-11 series record](../p2-11.md), split per installment.
> PR: [#5860](https://github.com/ubugeeei-prod/vize/pull/5860), merged
> 2026-09-06 as `99db931e5`.

This installment closes the P2-11 production-lane switch. The explicit
`VIZE_DAVINCI_DOM=legacy` selector, its `DOM_LANE_FLAG` name and the
`dom_lane_selection` / `DomLaneSelection` compatibility path were removed from
the live DOM compile surface, leaving S2 DOM emit as the production selector
for supported compiles.

The legacy-selector test now verifies ordinary S2 selection rather than an
environment override, and the SFC self-closing selector fixtures assert that
parser-recovered templates keep their S2 render path while preserving
compatibility diagnostics.

## Evidence

`tests/tooling/davinci-dom-production-boundary.test.ts` requires live DOM
production sources to stay free of `VIZE_DAVINCI_DOM`, `DOM_LANE_FLAG`,
`dom_lane_selection`, `DomLaneSelection` and direct environment reads.

`crates/vize_atelier_dom/tests/davinci_s2_legacy_selector.rs` and
`crates/vize_atelier_dom/tests/davinci_s2_sfc_self_closing_selector.rs` pin the
post-deletion selector behavior. `crates/vize_atelier_dom/src/compile.rs`,
`crates/vize_atelier_dom/src/compile/sfc.rs`,
`crates/vize_atelier_dom/src/compile/stage_options.rs`,
`crates/vize_s1_to_s2/src/emit.rs` and `crates/vize_s1_to_s2/src/lib.rs` carry
the code deletion.

## Completion

This installment ticks P2-11. The DOM output waiver budget remains zero, the
hydrated Real Project Matrix corpus evidence remains the canonical TS-25
record, and P2-20 still owns the later phase-exit review for differential-lane
retirement and other phase-wide gates.
