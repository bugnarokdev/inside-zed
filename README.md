# INSIDE ZED

**Under the hood of a modern Rust editor.**

<!-- stated-date:start -->
**Deep technical exploration**, last updated 26 November 2026. Stated for information only; it is not refreshed automatically.
<!-- stated-date:end -->

An independent, source-based exploration of [Zed](https://github.com/zed-industries/zed): its data structures, text engine, UI runtime, concurrency model, and performance trade-offs.

<p align="center">
  <strong>RUST ENGINEERING&nbsp; · &nbsp;DATA STRUCTURES&nbsp; · &nbsp;TEXT SYSTEMS&nbsp; · &nbsp;PERFORMANCE</strong>
</p>

---

> **Research, not marketing.** Every claim is classified as a source observation, an engineering interpretation, or an open hypothesis.

## Explore the engine

| Investigation | Focus | Status |
|---|---|---|
| [From zero to the numbers](docs/getting-started.html) | Pinned toolchain, sparse checkout, every command and what each one proves, and the boundaries of the study | Entry point |
| [Architecture map](docs/architecture.html) | Workspace layers and major subsystems | First pass |
| [Text engine](docs/text-engine.html) | Buffer, snapshots, fragments, history, and storage | In progress |
| [Edit lifecycle](docs/edit-lifecycle.html) | End-to-end trace: operations, fragments, anchors, undo, and remote replay | Deep dive |
| [Collaboration lab](docs/collaboration-lab.html) | Three replicas, divergent delivery orders, deferred operations, and convergence tests | Test trace |
| [Invariants & adversarial testing](docs/invariants-and-adversarial-testing.html) | Audit of structural checks, randomized convergence, known regressions, and the executed adversarial matrix | Engineering audit, 5 tests executed |
| [Measured, not guessed](docs/measurements.html) | Insert latency vs document size, offset lookup cost, undo grouping, and rope chunk geometry | Real run, raw output committed |
| [UI runtime & concurrency](docs/runtime.html) | The `!Send` local executor, weighted priority fairness, and Lamport ordering across replicas | Engineering audit, 5 tests executed |
| [Locator deep dive](docs/locator.html) | Stable ordering positions and insertion patterns | Initial notes |
| SumTree | Aggregated summaries, dimensions, cursors, and mutation | In progress |
| Rope | Chunk storage, bitmap indexes, and coordinate conversion | In progress |
| GPUI | Rendering, layout, events, and platform integration | Partly covered by the runtime audit |

**[Open the Inside Zed research site →](https://bugnarokdev.github.io/inside-zed/)**

The site is built from `site/` into `docs/` by `site/build.py`; `docs/` is generated, so edit `site/` and rebuild instead of patching the output. `.github/workflows/docs.yml` fails when the two drift apart. Every page carries a language selector: English is the default, French is a full translation loaded from `site/assets/i18n-fr.json`.

## Executed evidence

This repository does not only read Zed's source - it runs it. Four artefacts back the claims on the site:

**1. Adversarial concurrency tests** (`harness/tests/adversarial.rs`, `permutations.rs`) executed against the real `text` crate, each asserting Zed's own `Buffer::check_invariants()`. Includes the permutation case: three insertions at one cursor delivered in all six orders, converging to `"aCBAb"`, with tie-breaking traced to `Lamport::cmp` in `crates/clock/src/clock.rs`.

**2. Runtime tests** (`harness/tests/concurrency.rs`, `ui_runtime.rs`) against the `clock` and `scheduler` crates. The `clock` crate has no upstream tests of its own, so its ordering and version-vector semantics are covered here. The `scheduler` tests pin `Priority::weight()`, the deterministic `TestScheduler`, and the `Send`/`!Send` executor boundary.

**3. Data structure tests** (`harness/tests/data_structures.rs`) against `sum_tree` and `rope`. Fourteen tests measure the properties the site previously only described from source: a clone is independent of its original on write, the root summary equals the fold of the items, `extent` agrees with `summary`, seeking is bias-sensitive, and - the finding that contradicts the naive reading - `Point.column` and `line_len` count UTF-8 **bytes**, not characters, which is why `PointUtf16` exists as a separate type.

**4. Two measurement harnesses**: `harness/src/main.rs` linked against the pinned source produces the latencies published on the site (raw output of all three runs committed at `harness/results/release-run*.txt`), and `harness/src/bin/priority-draw.rs` measures the weighted priority draw against the real weights (output at `harness/results/priority-draw.txt`).

Totals: **33 tests, 33 passing**, plus the fairness measurement. Raw suite output is committed at `harness/results/suite-run.txt`.

### Reproducing

```sh
./tools/setup-zed.sh            # sparse, blob-filtered checkout of the pin
cd harness && cargo test --release          # 33 tests
cd harness && cargo run --release --bin priority-draw   # fairness measurement
cd harness && cargo build --release \
  && ./target/release/inside-zed-harness    # measurements
```

Requires network access to GitHub. Uses Rust 1.100.0, the same toolchain the pinned upstream commit declares in its own `rust-toolchain.toml`: `harness/rust-toolchain.toml` pins it, and all 33 tests build and pass on it with no nightly features required.

Everything is pinned to upstream `2664a0aa1eae5dabf8f8273c03f88d53061ca7dc`. Nothing in the upstream tree is modified: the harness links Zed's crates by path from a git-ignored `vendor/zed` checkout, and `.github/workflows/harness.yml` re-materialises that checkout on every run.

## Evidence standard

- **Verified** - directly supported by inspected source code or tests.
- **Analysis** - an interpretation of the implementation and its likely consequences.
- **Hypothesis** - a question that requires more source tracing, a targeted test, or a benchmark.

Performance claims require measurements. Complexity claims are labelled theoretical unless validated. Source links should be pinned to a commit when an investigation becomes stable.

## Research principles

1. **Source first.** Follow implementation, not just documentation.
2. **Trace end to end.** Connect APIs, mutation paths, data structures, and tests.
3. **Separate fact from inference.** Make uncertainty visible.
4. **Respect version context.** Record the upstream revision being studied.
5. **Measure before claiming.** Treat untested performance ideas as hypotheses.

## Independence

This is an independent engineering study. It is not affiliated with, endorsed by, or sponsored by Zed Industries. The upstream project is [zed-industries/zed](https://github.com/zed-industries/zed). Zed's source code, trademarks, and licenses remain with their respective owners. This repository contains original commentary and analysis; consult the upstream repository for the authoritative implementation.

---

<sub>INDEPENDENT ENGINEERING RESEARCH · RUST-FIRST · SOURCE-BASED</sub>
