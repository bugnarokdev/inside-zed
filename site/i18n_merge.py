#!/usr/bin/env python3
"""Assemble the per-page French dictionaries into one lookup table.

Each page dictionary is a flat list of English source strings in document order.
The FR lists must line up one-to-one with them. The output is a JSON object
keyed by the exact English string, which is what site.js looks up at runtime.
"""
import json
import sys
from pathlib import Path

SRC = Path("/tmp/i18n/src")
FR = Path("/tmp/i18n/fr")
OUT = Path("docs/assets/i18n-fr.json")
PAGES = [
    "common",
    "index",
    "architecture",
    "text-engine",
    "edit-lifecycle",
    "locator",
    "collaboration-lab",
    "invariants-and-adversarial-testing",
    "measurements",
    "runtime",
]

table: dict[str, str] = {}
problems: list[str] = []

for page in PAGES:
    src_path = SRC / f"{page}.json"
    fr_path = FR / f"{page}.json"
    if not src_path.exists():
        continue
    src = json.loads(src_path.read_text(encoding="utf-8"))
    if not fr_path.exists():
        problems.append(f"{page}: missing translation ({len(src)} strings)")
        continue
    fr = json.loads(fr_path.read_text(encoding="utf-8"))
    if len(fr) != len(src):
        problems.append(f"{page}: length mismatch src={len(src)} fr={len(fr)}")
        continue
    for en, f in zip(src, fr):
        if not isinstance(f, str) or not f.strip():
            problems.append(f"{page}: empty translation for {en[:50]!r}")
            continue
        if f != en:
            table[en] = f

if problems:
    print("PROBLEMS:")
    for p in problems:
        print("  -", p)
    if "--force" not in sys.argv:
        sys.exit(1)

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(
    json.dumps(table, ensure_ascii=False, indent=1, sort_keys=True),
    encoding="utf-8",
)
print(f"wrote {OUT} with {len(table)} entries")
