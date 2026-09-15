//! The Dota 2 logo block art shown to the left of the info panels.
//!
//! `art/dota2_block_art.txt` is a truecolor block-art render (captured raw,
//! ANSI escapes and all) of the Dota 2 logo, in the same style Omarchy's
//! fastfetch config uses for its distro logo. Embedded at build time so the
//! binary stays self-contained.
//!
//! Source: `art/Dota2.png` (the icon-only mark, no "DOTA 2" wordmark),
//! rendered via `chafa --format symbols --symbols block --size 52x26
//! --colors full --color-space rgb --dither none art/Dota2.png`, then
//! stripped of the leading/trailing `\x1b[?25l`/`\x1b[?25h` cursor-visibility
//! escapes chafa emits. Re-run that command and re-strip to regenerate.

use crate::ansi;

const RAW: &str = include_str!("../art/dota2_block_art.txt");

/// The art's lines, with the leading/trailing cursor-visibility escapes
/// (`\x1b[?25l` / `\x1b[?25h`) from the capture stripped - we don't want a
/// fetch tool touching cursor visibility itself.
pub fn lines() -> Vec<String> {
    RAW.replace("\x1b[?25l", "")
        .replace("\x1b[?25h", "")
        .lines()
        .map(str::to_string)
        .collect()
}

/// Visible column width of the art (all lines are rendered to the same
/// width by the capture tool).
pub fn width() -> usize {
    lines()
        .iter()
        .map(|l| ansi::visible_width(l))
        .max()
        .unwrap_or(0)
}
