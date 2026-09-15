//! fastfetch-style output: the Dota 2 logo block art on the left, labeled
//! sections (thin title rule + tree connectors, matching Omarchy's
//! fastfetch look) on the right.

use std::io::{IsTerminal, Write};

use crate::ansi;
use crate::art;
use crate::heroes;
use crate::image_logo;
use crate::stats::Stats;

/// Like `println!`, but ignores write errors instead of panicking - a
/// fetch tool is routinely piped into `head`/`less`, which closes the pipe
/// early (EPIPE) and would otherwise crash us with a broken-pipe panic.
macro_rules! outln {
    () => {{ let _ = writeln!(std::io::stdout()); }};
    ($($arg:tt)*) => {{ let _ = writeln!(std::io::stdout(), $($arg)*); }};
}

const GAP: &str = "  "; // columns between the logo and the info panel

struct Palette {
    dim: &'static str,
    reset: &'static str,
    bold: &'static str,
    green: &'static str,
    red: &'static str,
}

impl Palette {
    fn detect() -> Self {
        let enabled =
            std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        if enabled {
            Palette {
                dim: "\x1b[2m",
                reset: "\x1b[0m",
                bold: "\x1b[1m",
                green: "\x1b[32m",
                red: "\x1b[31m",
            }
        } else {
            Palette {
                dim: "",
                reset: "",
                bold: "",
                green: "",
                red: "",
            }
        }
    }
}

struct Section {
    title: String,
    lines: Vec<String>,
}

fn rule_line(pal: &Palette, title: &str, width: usize) -> String {
    let dashes = width.saturating_sub(ansi::visible_width(title));
    let left = dashes / 2;
    let right = dashes - left;
    format!(
        "{}{}{title}{}{}",
        pal.dim,
        "─".repeat(left),
        "─".repeat(right),
        pal.reset,
    )
}

/// Renders one section as `["<rule with title>", "  first line", "├ ...",
/// ..., "└ last line"]`, all padded to `width` visible columns.
fn render_section(pal: &Palette, section: &Section, width: usize) -> Vec<String> {
    let mut out = vec![rule_line(pal, &section.title, width)];
    let last = section.lines.len().saturating_sub(1);
    for (i, line) in section.lines.iter().enumerate() {
        let connector = if i == 0 {
            "  ".to_string()
        } else if i == last {
            format!("{}└{} ", pal.dim, pal.reset)
        } else {
            format!("{}├{} ", pal.dim, pal.reset)
        };
        out.push(format!("{connector}{line}"));
    }
    out
}


fn build_sections(pal: &Palette, stats: &Stats) -> Vec<Section> {
    let mut sections = Vec::new();

    let lifetime = &stats.lifetime;
    let lifetime_lines = vec![
        format!("Matches: {}", lifetime.total()),
        format!(
            "Wins:    {}{}{} ({:.1}%)",
            pal.green,
            lifetime.wins,
            pal.reset,
            lifetime.winrate()
        ),
        format!("Losses:  {}{}{}", pal.red, lifetime.losses, pal.reset),
    ];
    sections.push(Section {
        title: "Lifetime".to_string(),
        lines: lifetime_lines,
    });

    if !stats.top_heroes.is_empty() {
        sections.push(Section {
            title: "Top Heroes".to_string(),
            lines: stats
                .top_heroes
                .iter()
                .take(5)
                .map(|h| {
                    format!(
                        "{:<16} {}{:>3}W{} {}{:>3}L{} ({:>5.1}%)",
                        heroes::name(h.hero_id),
                        pal.green,
                        h.wins,
                        pal.reset,
                        pal.red,
                        h.losses,
                        pal.reset,
                        h.winrate()
                    )
                })
                .collect(),
        });
    }

    if let Some(conduct) = &stats.conduct {
        sections.push(Section {
            title: "Conduct".to_string(),
            lines: vec![
                format!("Behavior: {}", conduct.behavior_rating),
                format!("Commends: {}{}{}", pal.green, conduct.commend_count, pal.reset),
                format!(
                    "Reports:  {}{}{}",
                    if conduct.reports_count > 0 { pal.red } else { "" },
                    conduct.reports_count,
                    pal.reset
                ),
                format!(
                    "Recent matches clean/abandoned: {}/{}",
                    conduct.matches_clean, conduct.matches_abandoned
                ),
            ],
        });
    }

    sections
}

/// Prints `right_lines` with the Dota 2 logo art to their left, row by row.
fn print_with_logo(right_lines: &[String]) {
    let logo = art::lines();
    let logo_width = art::width();
    let rows = logo.len().max(right_lines.len());

    for i in 0..rows {
        let logo_line = logo.get(i).map(String::as_str).unwrap_or("");
        let logo_pad = logo_width.saturating_sub(ansi::visible_width(logo_line));
        let right_line = right_lines.get(i).map(String::as_str).unwrap_or("");
        outln!("{logo_line}{}{GAP}{right_line}", " ".repeat(logo_pad));
    }
}

/// Small downward nudge so the logo doesn't look top-flush against the
/// header line - just enough to read as vertically centered next to it.
const IMAGE_ROW_OFFSET: u32 = 1;

/// Prints `right_lines` overlaid on the real Dota 2 logo image, using the
/// Kitty graphics protocol. The image is placed without moving the cursor
/// (`image_logo::print`'s `C=1`), then each line is positioned relative to
/// that same start point (saved once via DECSC) so text and image share
/// their rows without either one disturbing the other.
fn print_with_image_logo(right_lines: &[String]) {
    let rows = right_lines.len().max(1) as u32;
    let mut stdout = std::io::stdout();

    let _ = write!(stdout, "\x1b7"); // DECSC: save cursor as our anchor
    let _ = write!(stdout, "\x1b[{IMAGE_ROW_OFFSET}B"); // nudge the logo down
    let cols = image_logo::print(rows);
    let indent = cols + 2; // logo width + gap

    for (i, line) in right_lines.iter().enumerate() {
        let _ = write!(stdout, "\x1b8"); // DECRC: back to the anchor
        if i > 0 {
            let _ = write!(stdout, "\x1b[{i}B");
        }
        let _ = write!(stdout, "\x1b[{indent}C{line}");
    }

    // Leave the cursor on a fresh line below both the image and the text.
    let _ = writeln!(stdout, "\x1b8\x1b[{}B", rows + IMAGE_ROW_OFFSET);
    let _ = stdout.flush();
}

/// Centers `text` within `width` visible columns.
fn center(text: &str, width: usize) -> String {
    let pad = width.saturating_sub(ansi::visible_width(text));
    let left = pad / 2;
    let right = pad - left;
    format!("{}{text}{}", " ".repeat(left), " ".repeat(right))
}

pub fn print(persona_name: &str, stats: &Stats) {
    let pal = Palette::detect();
    let sections = build_sections(&pal, stats);

    // Width shared by every section's rule and content, so right edges line
    // up the way Omarchy's fastfetch config does. Content lines get a
    // 2-column connector prefix ("├ " / "└ "), so budget for that too.
    let width = sections
        .iter()
        .flat_map(|s| {
            std::iter::once(ansi::visible_width(&s.title))
                .chain(s.lines.iter().map(|l| ansi::visible_width(l) + 2))
        })
        .max()
        .unwrap_or(20);
    let header = format!("{}{}{}", pal.bold, center(persona_name, width), pal.reset);

    let mut right = vec![header, String::new()];
    for section in &sections {
        right.extend(render_section(&pal, section, width));
        right.push(String::new());
    }
    right.pop(); // drop the trailing blank line

    if image_logo::supported() {
        print_with_image_logo(&right);
    } else {
        print_with_logo(&right);
    }
}
