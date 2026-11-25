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
| [Locator deep dive](docs/locator.html) | Stable ordering positions and insertion patterns | Initial notes |
| SumTree | Aggregated summaries, dimensions, cursors, and mutation | In progress |
| Rope | Chunk storage, bitmap indexes, and coordinate conversion | In progress |
| Collaboration | Replicas, clocks, operation ordering, and convergence | Partial trace |
| GPUI | Rendering, layout, events, and platform integration | To investigate |

**[Open the Inside Zed research site →](https://bugnarokdev.github.io/inside-zed/)**

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
