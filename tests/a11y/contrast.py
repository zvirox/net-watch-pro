#!/usr/bin/env python3
"""Check DESIGN.md §9.1 contrast pairs against the frozen Linear token values.

No dependencies: parses app/src/tailwind.css with the standard library
and computes WCAG 2.2 relative luminance. Run from the repo root:

    python3 tests/a11y/contrast.py

Exits 1 (with details) when any pair fails, so CI fails on token regressions.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = ROOT / "app" / "src" / "tailwind.css"

PAIRS: list[tuple[str, str, float]] = [
    ("text", "bg-app", 12.0),
    ("text", "bg-surface", 12.0),
    ("text-muted", "bg-surface", 4.5),
    ("on-accent", "accent-blue", 4.5),
    ("tint-blue-fg", "bg-surface", 4.5),
    ("tint-blue-fg", "tint-blue", 4.5),
    ("tint-green-fg", "bg-surface", 4.5),
    ("tint-green-fg", "tint-green", 4.5),
    ("tint-red-fg", "bg-surface", 4.5),
    ("tint-red-fg", "tint-red", 4.5),
    ("tint-amber-fg", "bg-surface", 4.5),
    ("tint-amber-fg", "tint-amber", 4.5),
    ("tint-sky-fg", "bg-surface", 4.5),
    ("tint-teal-fg", "bg-surface", 4.5),
]


def parse_block(text: str, marker: str) -> dict[str, str]:
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"missing token block: {marker}")
    end = text.find("\n}", start)
    block = text[start:end if end >= 0 else len(text)]
    return dict(re.findall(r"--([\w-]+):\s*(#[0-9a-fA-F]{6})\b", block))


def luminance(hex_color: str) -> float:
    channels = [int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels]
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]


def ratio(a: str, b: str) -> float:
    light, dark = sorted((luminance(a), luminance(b)), reverse=True)
    return (light + 0.05) / (dark + 0.05)


def main() -> int:
    text = TOKENS.read_text(encoding="utf-8")
    shared = parse_block(text, ":root {")
    themes = {
        "dark": {**shared, **parse_block(text, "[data-theme='dark'],\n:root.dark {")},
        "light": {**shared, **parse_block(text, "[data-theme='light'],\n:root.light {")},
    }
    failures = 0
    for theme, tokens in themes.items():
        print(f"[{theme}]")
        for fg, bg, minimum in PAIRS:
            if fg not in tokens or bg not in tokens:
                print(f"  MISSING {fg} / {bg}")
                failures += 1
                continue
            measured = ratio(tokens[fg], tokens[bg])
            ok = measured >= minimum
            print(
                f"  {'PASS' if ok else 'FAIL'} {fg:20s} on {bg:14s} "
                f"{measured:5.2f}:1 (min {minimum:.1f})"
            )
            if not ok:
                failures += 1
    if failures:
        print(f"\n{failures} contrast pair(s) failed — DESIGN.md §9.1")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
