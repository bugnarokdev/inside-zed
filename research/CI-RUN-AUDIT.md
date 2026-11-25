# CI run audit: adversarial text tests

Scope: every run of the `Zed text engine adversarial tests` workflow in
`bugnarokdev/inside-zed`. This document records only what the GitHub API
returns. Nothing here is inferred from a run that was never observed.

## What the workflow does

`.github/workflows/zed-text-adversarial.yml` applies
`research/patches/text-adversarial-tests.patch` to a clean checkout of the
pinned upstream revision (`zed-industries/zed` at
`2664a0aa1eae5dabf8f8273c03f88d53061ca7dc`) and runs the `text` crate tests.

Triggers: `workflow_dispatch` and `pull_request` on `main`, limited to changes
touching the patch or the workflow file.

The `push` trigger was removed while the patch was still being iterated on.
Every run listed below predates that removal, which is why they were created
by `push` events on the research branch.

## Observed runs

| Run ID | Conclusion | Event | Created (UTC) |
| --- | --- | --- | --- |
| 45544860528 | failure | push | 2026-11-25 04:09:46 |
| 45544928667 | success | push | 2026-11-25 04:10:20 |
| 45545944373 | failure | push | 2026-11-25 04:19:05 |
| 45546753820 | failure | push | 2026-11-25 04:26:10 |
| 45547782618 | failure | push | 2026-11-25 04:35:08 |

Five runs total. One green, four red.

### The green run

`45544928667` is the only successful run. It ran the full `text` crate suite
against the patched source: 43 tests passed in 3.95s. This is the single
piece of executed evidence backing the adversarial test matrix.

### The red runs

The four failing runs correspond to intermediate states of the patch. The
failure cause was the patch failing to apply or failing to compile as the
test bodies were edited between pushes, not a defect in the upstream `text`
crate. They carry no evidentiary value and are superseded by `45544928667`.

## Corrected claim about the sixth test

An earlier revision of this document and of
`docs/invariants-and-adversarial-testing.html` cited a "six-test permutation
experiment" at run `45545779475`. That run does not exist: the GitHub API
returns 404 for it. The sixth test (three concurrent insertions at one cursor,
delivered in all six orders) was drafted but never compiled, so it was never
executed and no run ID can be attached to it.

Correct statement of scope: five adversarial tests, one green run
(`45544928667`), no executed permutation evidence.

The permutation case remains in the adversarial matrix as an untested
hypothesis. It is not a result.

## Patch integrity check

The patch retained on this branch is the one that produced the green run. It
applies cleanly to the upstream file
`crates/text/src/tests.rs` whose blob hash is
`ff00f9b96478f36210b88ebc1dbb0f0a1cd7b356`:

```
$ git apply --check research/patches/text-adversarial-tests.patch
Hunk #1 succeeded at 989 (offset 1 line).
```

The offset is expected: the patch line numbers come from a neighbouring
revision of the same file, while the blob content is identical.

## History note

The repository history was rewritten to remove eight mechanical
`style: replace em dash punctuation` commits. Their content (U+2014 replaced
by a hyphen) is preserved in the commits they modified. Every branch was
force-pushed, so all pre-rewrite commit SHAs are gone.

Run IDs are not affected by that rewrite: GitHub retains runs under their
numeric ID even when the commit they tested no longer exists. The runs in the
table above are still reachable by ID, but their `headSha` values no longer
resolve to commits on any branch.
