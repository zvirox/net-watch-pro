#!/usr/bin/env python3
"""Check DESIGN.md §9.1 contrast pairs against the shipped token values.

No dependencies: parses app/src/lib/theme/tokens.css with the standard library
and computes WCAG 2.2 relative luminance. Run from the repo root:

    python3 tests/a11y/contrast.py

Exits 1 (with details) when any pair fails, so CI fails on token regressions.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = ROOT / "app" / "src" / "lib" / "theme" / "tokens.css"

# (token, background token, minimum ratio)
PAIRS: list[tuple[str, str, float]] = [
    ("text", "background", 12.0),
    ("text", "surface", 12.0),
    ("text-secondary", "surface", 4.5),
    ("on-primary", "primary", 4.5),
    ("primary", "surface", 4.5),
    ("success", "surface", 4.5),
    ("warning", "surface", 4.5),
    ("error", "surface", 4.5),
    ("text-disabled", "surface", 3.0),
    ("border-strong", "surface", 3.0),
    ("chart-1", "surface", 3.0),
    ("chart-2", "surface", 3.0),
    ("chart-3", "surface", 3.0),
    ("chart-4", "surface", 3.0),
    ("chart-5", "surface", 3.0),
    ("chart-muted", "surface", 3.0),
    ("chart-axis-label", "surface", 4.5),
]


def parse_theme(text: str, marker: str) -> dict[str, str]:
    """Parse `--nwp-*` hex values inside one CSS rule block."""
    start = text.find(marker)
    if start < 0:
        raise SystemExit(f"missing theme block: {marker}")
    end = text.find("\n}", start)
    block = text[start:end if end >= 0 else len(text)]
    values: dict[str, str] = {}
    for name, value in re.findall(r"--nwp-([\w-]+):\s*(#[0-9a-fA-F]{6})\b", block):
        values[name] = value
    return values


def luminance(hex_color: str) -> float:
    channels = [int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels]
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]


def ratio(a: str, b: str) -> float:
    light, dark = sorted((luminance(a), luminance(b)), reverse=True)
    return (light + 0.05) / (dark + 0.05)


def main() -> int:
    text = TOKENS.read_text(encoding="utf-8")
    themes = {
        "light": parse_theme(text, ":root {"),
        "dark": parse_theme(text, "[data-theme='dark'] {"),
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
                f"  {'PASS' if ok else 'FAIL'} {fg:20s} on {bg:12s} "
                f"{measured:5.2f}:1 (min {minimum:.1f})"
            )
            if not ok:
                failures += 1
    if failures:
        print(f"\n{failures} contrast pair(s) failed — DESIGN.md §9.1")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
