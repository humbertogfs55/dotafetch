# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`dotafetch` is a fastfetch-style CLI that prints Dota 2 account stats (lifetime W/L, top heroes, conduct) to the terminal. It reads Dota 2's local stat cache directly — no Steam login, no API key, no network calls at runtime. All Dota reference data (hero names) is vendored into the binary at compile time from `data/heroes.json`.

Requires Dota 2 to have been run at least once on the machine, so the client has written its local `stats.dat`.

## Commands

```
cargo build --release       # release build
./target/release/dotafetch  # run it
cargo build                 # debug build
cargo test                  # run all tests (each module has inline #[cfg(test)] unit tests)
cargo test -p dotafetch vbkv::tests::rejects_bad_magic   # run a single test
cargo fmt
cargo clippy
```

Debugging helper: `dotafetch dump <path>` parses any local VBKV `.dat` file (`stats.dat`, `last_match.dat`, etc.) and prints it as pretty JSON — useful for exploring the on-disk format without writing a one-off script.

## Architecture

Pipeline, in `main.rs`: **locate** the account → **build stats** from its files → **render** to the terminal.

- `locate.rs` — finds the Steam install (`$STEAM_ROOT`, `~/.steam/root`, `~/.local/share/Steam`), confirms Dota 2 (app id `570`) is installed by parsing `steamapps/libraryfolders.vdf`, then reads `config/loginusers.vdf` to pick the most-recently-logged-in account (highest `Timestamp`). Converts that account's SteamID64 to SteamID32 (`- 76561197960265728`) to build the path `userdata/<id32>/570/remote/cfg/`, which is where `stats.dat` lives. Also best-effort resolves the Dota install's own `game/dota/cfg` dir (via `appmanifest_570.acf`) for the optional conduct file — this one is never required, since conduct data lives beside the game install rather than in cloud-synced userdata.
- `stats.rs` — reads `stats.dat` (via `vbkv`) and walks `Stats.hero_standings.standings` to build lifetime totals and per-hero win/loss records, sorted by games played. `hero_id == 0` is a placeholder bucket in the local cache: counted in lifetime totals but excluded from the per-hero list. Separately, best-effort parses the plain-text `latest_conduct_*.txt` (whitespace-separated `key: value` tokens, most-recently-modified file wins) for the optional Conduct section.
- `vdf.rs` / `vbkv.rs` — two independent, from-scratch parsers for Valve's two KeyValues encodings: `vdf` is the plain-text format (`"key" "value"`, nested `{ }` blocks, `//` comments) used by `loginusers.vdf`/`libraryfolders.vdf`/`.acf` manifests; `vbkv` is the binary format (magic `VBKV`, typed tag bytes: 0x00 dict / 0x01 string / 0x02 i32 / 0x03 f32 / 0x07 u64, 0x0B dict terminator) used by the account's `.dat` files. Neither depends on the other; `locate.rs` uses `vdf`, `stats.rs` uses `vbkv`.
- `heroes.rs` — hero id → name lookup, lazily built once (`OnceLock`) from `data/heroes.json` (vendored from `odota/dotaconstants`), embedded via `include_str!`. Unknown ids fall back to `"Hero <id>"` rather than erroring — this table drifts behind newly-released heroes over time.
- `render.rs` — builds labeled `Section`s (title rule + `├`/`└` tree-connector lines, matching Omarchy's fastfetch look) from `Stats`, computes a shared content width so every section's right edge aligns, then lays the sections out beside the logo. Color/dim/bold codes come from `Palette::detect()`, which no-ops everything when stdout isn't a TTY or `NO_COLOR` is set.
- `art.rs` — renders the Dota 2 logo from `art/dota2_block_art.txt`, a pre-captured truecolor block-art render, embedded via `include_str!`. (A Kitty-graphics-protocol image renderer existed briefly but was removed: overlaying text on a non-cursor-advancing image via cursor-anchored positioning doesn't scroll correctly when the cursor starts near the bottom of the terminal — the image gets clipped and later text lines overwrite each other. Plain sequential `\n`-terminated output, as `art.rs`/`render.rs` use, scrolls correctly in that case, so the block-art path is now the only renderer.)
- `ansi.rs` — `visible_width()`, used everywhere text needs to be padded/centered/aligned around embedded ANSI escape codes.

### Key invariants worth knowing before editing

- All file reads (Steam config, `.dat` files, conduct file) are best-effort or produce a typed error (`LocateError`, `StatsError`) with a clear message — this is a fetch tool that should fail loudly and immediately on missing required data (`main.rs` prints the error and exits), but silently skip optional data (conduct box, install dir).
- `render.rs`'s `outln!` macro swallows write errors deliberately: this tool is routinely piped into `head`/`less`, which closes the pipe early (EPIPE) and must not crash it.
- Width/alignment math throughout `render.rs`/`art.rs` always goes through `ansi::visible_width`, never `str::len` or `.chars().count()`, to stay correct with embedded color codes.
