# Dota 2 nerd-font icon pack — investigation & plan

Status: investigation only, nothing implemented yet. This is a build plan for
a *separate*, later project (`dota2-nerd-font/`, working title) that turns
the locally-sourced hero minimap icons in `reference/hero_minimap_icons/`
(see `scripts/fetch_reference_icons.sh`) into an original, hand-cleaned
monochrome glyph set, compiled into a standalone custom font (not a patch of
an existing Nerd Font). The source PNGs stay reference-only and are never
shipped as-is; only hand-redrawn/cleaned glyph outlines go into the font.

## Proposed pipeline

1. **Source**: `reference/hero_minimap_icons/*.png` — 128 icons, all 32×32
   `TrueColorAlpha` PNGs, ~290 distinct colors each (anti-aliased full-color
   art, not flat pre-vectorized icons). Confirmed via `identify -verbose`.

2. **Raster → SVG (auto-trace, guide layer only)**
   - Ran a real trace test (not guessed): the npm `potrace` port (JS port of
     the potrace algorithm, installs with no `sudo` — verified) against 6
     sample icons at its default settings (`threshold: 128, turdSize: 2,
     optTolerance: 0.2`).
   - Result: potrace emits one compound `<path>` per icon, but each is made
     of **4–11 disjoint subpaths** (`M` commands) and **27–65 cubic curve
     segments**. That confirms the risk already flagged in the plan
     ("monochrome needs to be tested"): naive thresholding fragments the
     anti-aliased source into several small disconnected blobs rather than
     one clean silhouette.
   - **Conclusion**: don't treat auto-trace output as font-ready. Use it as
     a low-opacity tracing guide and hand-clean/redraw each glyph on top —
     consistent with "will not look the same at all" from the original
     scoping conversation. Budget real per-glyph time, not a batch script.
   - Threshold/`turdSize` tuning per icon *before* tracing may still reduce
     cleanup work and is worth a pass (some icons will binarize cleaner than
     others), but shouldn't be assumed to fully solve it.

3. **Normalize (bounding box / baseline / scale)**
   - Don't need bespoke tooling for this: `fantasticon` (see below) has
     `--font-height`, `--normalize` (scales every icon to the tallest), and
     `--descent` built in. As long as hand-cleaned glyphs use a consistent
     `viewBox` convention, this stage is close to free.
   - If finer control turns out to be needed, `svgo` (npm) can normalize
     `viewBox`/bounds per file as a pre-pass.

4. **SVG set → TTF**
   - **Recommended: `fantasticon`** (npm, verified installable and runnable
     here without `sudo`). Takes a folder of SVGs, emits `ttf`/`woff`/`woff2`
     plus `css`/`json` maps in one call. Confirmed by reading its packaged
     config (not from memory) that it supports:
     ```js
     codepoints: { 'abaddon': 0xf0100, /* ... */ }
     ```
     — i.e. an explicit per-icon custom codepoint map, which is exactly
     "assign each hero a PUA codepoint."
   - **Fallback: `fontforge` + its Python API** (`extra/fontforge`, pacman,
     needs `sudo` — not installed in this investigation). The traditional,
     more manual/scriptable route with finer glyph-metric control. Reach for
     this only if `fantasticon`'s control proves insufficient.

5. **PUA codepoint planning — still open, needs a check before implementing**
   - The plan is a standalone `.ttf`, not a patch — presumably layered via
     per-codepoint terminal font fallback (Kitty `symbol_map`, Alacritty,
     foot) alongside JetBrainsMono Nerd Font.
   - For that to work without glyph collisions, the chosen codepoint block
     must **not** overlap ranges Nerd Fonts itself already uses. Nerd Fonts
     occupies large parts of the BMP PUA (`U+E000`–`U+F8FF`) across its
     bundled sets (Seti, Devicons, Font Awesome, Material, Weather, Octicons,
     etc.), plus parts of the supplementary PUA-A plane (`U+F0000`+) in v3.
   - **Action item before implementation**: pull the authoritative
     `glyphnames.json` from the nerd-fonts repo (or inspect an installed
     `ttf-*-nerd` package, e.g. `ttf-hack-nerd` is already on this machine
     via pacman) and pick a contiguous free block of 128+ codepoints outside
     every existing range — most likely somewhere in PUA-B (`U+100000`–
     `U+10FFFD`), which Nerd Fonts uses far more sparingly than the BMP PUA.
     Don't hand-guess a range; this is cheap to verify and easy to get wrong.

## Verified local toolchain (this machine)

Already present, no install needed:
- `python3`, `jq`, `curl`, `node`/`npm` (via mise), ImageMagick (`convert`/`magick`)

Installable without `sudo` (verified reachable on the npm registry):
- `potrace` (JS port), `sharp`, `fantasticon`, `svgicons2svgfont`, `svg2ttf`, `svgo`

Installable with `sudo` (pacman `extra/`, not installed — left for explicit
approval before the build phase, since it's a system-level change):
- `potrace` (system CLI, PBM/PGM/PPM/BMP input only — would need an
  ImageMagick bitmap pre-pass), `fontforge` (+ Python bindings), `inkscape`
  (useful either way, for the manual glyph-cleanup pass)

## Proposed repo layout

A new, separate project — not nested inside `dotafetch` — since it's an
unrelated font-build tool:

```
dota2-nerd-font/
├── fonts/
│   └── Dota2Icons.ttf        # fantasticon output
├── glyphs/                   # final, hand-cleaned/redrawn glyphs (font-ready)
│   ├── abaddon.svg
│   └── ...
├── trace/                    # gitignored - raw potrace output, guide layer only
│   ├── abaddon.svg
│   └── ...
├── mapping.json               # hero -> PUA codepoint, feeds fantasticon's `codepoints` config
├── scripts/
│   └── build_font.sh          # trace -> (manual cleanup) -> normalize -> fantasticon build
└── README.md
```

## Open questions for whoever picks this up

- Exact PUA block (see codepoint planning above) — needs the glyphnames.json
  check before `mapping.json` can be written.
- How much per-icon threshold tuning is worth doing before hand-cleanup vs.
  just tracing at one fixed setting and cleaning everything by hand anyway.
- Whether persona/alt-skin variants (e.g. the "Axe (Dark Carnival)" entry
  pulled down alongside the base roster) get their own codepoints or are
  dropped — the base roster (one icon per hero) is probably the actual
  target, so `reference/hero_minimap_icons/` should likely be filtered down
  before glyph work starts.
