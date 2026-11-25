# INSIDE ZED

**Under the hood of a modern Rust editor.**

An independent, source-based exploration of [Zed](https://github.com/zed-industries/zed): its data structures, text engine, UI runtime, concurrency model, and performance trade-offs.

<p align="center">
  <strong>RUST ENGINEERING&nbsp; · &nbsp;DATA STRUCTURES&nbsp; · &nbsp;TEXT SYSTEMS&nbsp; · &nbsp;PERFORMANCE</strong>
</p>

---

> **Research, not marketing.** Every claim is classified as a source observation, an engineering interpretation, or an open hypothesis.

## Explore the engine

| Investigation | Focus | Status |
|---|---|---|
| [Architecture map](docs/architecture.html) | Workspace layers and major subsystems | First pass |
| [Text engine](docs/text-engine.html) | Buffer, snapshots, fragments, history, and storage | In progress |
| [Edit lifecycle](docs/edit-lifecycle.html) | End-to-end trace: operations, fragments, anchors, undo, and remote replay | Deep dive |
| [Collaboration lab](docs/collaboration-lab.html) | Three replicas, divergent delivery orders, deferred operations, and convergence tests | Test trace |
| [Invariants & adversarial testing](docs/invariants-and-adversarial-testing.html) | Audit of structural checks, randomized convergence, known regressions, and the executed adversarial matrix | Engineering audit, 6 tests executed |
| [Measured, not guessed](docs/measurements.html) | Insert latency vs document size, offset lookup cost, undo grouping, and rope chunk geometry | Real run, raw output committed |
| [Locator deep dive](docs/locator.html) | Stable ordering positions and insertion patterns | Initial notes |
| SumTree | Aggregated summaries, dimensions, cursors, and mutation | In progress |
| Rope | Chunk storage, bitmap indexes, and coordinate conversion | In progress |
| GPUI | Rendering, layout, events, and platform integration | To investigate |

**[Open the Inside Zed research site →](https://bugnarokdev.github.io/inside-zed/)**

## Executed evidence

This repository does not only read Zed's source - it runs it. Two artefacts back the claims on the site:

**1. Six adversarial concurrency tests** (`harness/tests/`) executed against the real `text` crate, each asserting Zed's own `Buffer::check_invariants()`. Includes the permutation case: three insertions at one cursor delivered in all six orders, converging to `"aCBAb"`, with tie-breaking traced to `Lamport::cmp` in `crates/clock/src/clock.rs`.

**2. A measurement harness** (`harness/src/main.rs`) linked against the pinned source, producing the latencies published on the site. Raw run output is committed at `harness/results/release-run.txt`.

### Reproducing

```sh
./tools/setup-zed.sh            # sparse, blob-filtered checkout of the pin
cd harness && cargo test --release          # 6 tests
cd harness && cargo build --release \
  && ./target/release/inside-zed-harness    # measurements
```

Requires network access to GitHub. Uses Rust 1.100.0, the same toolchain the pinned upstream commit declares in its own `rust-toolchain.toml`: `harness/rust-toolchain.toml` pins it, and all six tests build and pass on it with no nightly features required.

Everything is pinned to upstream `2664a0aa1eae5dabf8f8273c03f88d53061ca7dc`. Nothing in the upstream tree is modified: the harness links Zed's crates by path, and the CI job applies a patch to a throwaway checkout.

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
