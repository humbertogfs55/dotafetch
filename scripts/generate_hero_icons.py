#!/usr/bin/env python3
"""Regenerates data/hero_icons.json - a hero_id -> 5-row truecolor
half-block ANSI render of that hero's minimap icon, downsampled to 9x5 via
`chafa`, embedded at compile time by src/hero_icons.rs (include_str!, same
pattern as data/heroes.json / src/heroes.rs).

Source: reference/hero_minimap_icons/*.png (see fetch_reference_icons.sh).
That directory is gitignored/local-only; only this 9x5 downsampled ANSI
derivative is vendored into the repo.

Requires `chafa` on PATH. Safe to re-run after new heroes ship or after
re-running fetch_reference_icons.sh - heroes with no matching icon file are
just skipped (rendered as blank space at runtime, same "drifts behind new
releases" fallback heroes.rs already uses for names).
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HEROES_JSON = os.path.join(ROOT, "data", "heroes.json")
ICON_DIR = os.path.join(ROOT, "reference", "hero_minimap_icons")
OUT_JSON = os.path.join(ROOT, "data", "hero_icons.json")

SIZE = "9x5"

# odota's hero name for this id doesn't match Liquipedia's mapicon filename
# for the same hero (renamed hero / different spacing) - map explicitly
# rather than guessing.
NAME_OVERRIDES = {
    "76": "Outworld Destroyer",  # odota: "Outworld Devourer" (old name)
    "131": "Ringmaster",  # odota: "Ring Master" (has a space, file doesn't)
}

CURSOR_RE = re.compile(r"\x1b\[\?25[lh]")


def icon_filename(hero_id: str, name: str) -> str:
    name = NAME_OVERRIDES.get(hero_id, name)
    name = name.replace(" ", "_").replace("'", "%27")
    return f"{name}_mapicon_dota2_gameasset.png"


def render(path: str) -> list[str]:
    out = subprocess.run(
        [
            "chafa",
            "--format", "symbols",
            "--symbols", "block",
            "--size", SIZE,
            "--colors", "full",
            "--color-space", "rgb",
            "--dither", "none",
            path,
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    out = CURSOR_RE.sub("", out)
    return out.splitlines()


def main() -> int:
    if subprocess.run(["which", "chafa"], capture_output=True).returncode != 0:
        print("generate_hero_icons.py: chafa is required (not found on PATH)", file=sys.stderr)
        return 1

    heroes = json.load(open(HEROES_JSON))
    icons: dict[str, list[str]] = {}
    missing = []

    for hero_id, name in sorted(heroes.items(), key=lambda kv: int(kv[0])):
        fname = icon_filename(hero_id, name)
        path = os.path.join(ICON_DIR, fname)
        if not os.path.isfile(path):
            missing.append((hero_id, name, fname))
            continue
        icons[hero_id] = render(path)

    with open(OUT_JSON, "w") as f:
        json.dump(icons, f, ensure_ascii=False, separators=(",", ":"))
        f.write("\n")

    print(f"wrote {len(icons)} icons to {os.path.relpath(OUT_JSON, ROOT)}")
    if missing:
        print(f"{len(missing)} heroes skipped (no matching reference icon):", file=sys.stderr)
        for hero_id, name, fname in missing:
            print(f"  {hero_id}\t{name}\t(expected {fname})", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
