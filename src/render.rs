//! fastfetch-style output: the Dota 2 logo block art on the left, labeled
//! sections (thin title rule + tree connectors, matching Omarchy's
//! fastfetch look) on the right.

use std::io::{IsTerminal, Write};

use crate::ansi;
use crate::art;
use crate::hero_icons;
use crate::heroes;
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
        let enabled = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
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

/// Top border of a section's box: `┌──title──┐`, `width` visible columns
/// between the corners.
fn top_border(pal: &Palette, title: &str, width: usize) -> String {
    let dashes = width.saturating_sub(ansi::visible_width(title));
    let left = dashes / 2;
    let right = dashes - left;
    format!(
        "{}┌{}{}{title}{}{}┐{}",
        pal.dim,
        "─".repeat(left),
        pal.reset,
        pal.dim,
        "─".repeat(right),
        pal.reset,
    )
}

/// Bottom border of a section's box: `└──────┘`, `width` visible columns
/// between the corners.
fn bottom_border(pal: &Palette, width: usize) -> String {
    format!("{}└{}┘{}", pal.dim, "─".repeat(width), pal.reset)
}

/// Renders one section as a bordered box: a `top_border`, then the content
/// lines (first line unprefixed, middle lines behind `│ ├`, the last line
/// behind `└ └` - the box's left edge and the tree's own corner turning
/// together), then a `bottom_border`. All content lines are padded to
/// `width` visible columns.
fn render_section(pal: &Palette, section: &Section, width: usize) -> Vec<String> {
    let mut out = vec![top_border(pal, &section.title, width)];
    let last = section.lines.len().saturating_sub(1);
    for (i, line) in section.lines.iter().enumerate() {
        let prefix = if i == 0 {
            "    ".to_string()
        } else if i == last {
            format!("{}└ └{} ", pal.dim, pal.reset)
        } else {
            format!("{}│ ├{} ", pal.dim, pal.reset)
        };
        out.push(format!("{prefix}{line}"));
    }
    out.push(bottom_border(pal, width));
    out
}

// Nerd Font glyph placeholders, one per stat line - picked to fit each
// stat for now; swap these out once the icon-pack feature lands. Written as
// `\u{...}` escapes (rather than literal glyphs) so the exact codepoint is
// unambiguous in source.
const ICON_MATCHES: &str = "\u{f11b}"; // fa-gamepad
const ICON_WINS: &str = "\u{f091}"; // fa-trophy
const ICON_LOSSES: &str = "\u{f057}"; // fa-times-circle
const ICON_HERO: &str = "\u{f007}"; // fa-user
const ICON_BEHAVIOR: &str = "\u{f21e}"; // fa-heartbeat
const ICON_COMMENDS: &str = "\u{f087}"; // fa-thumbs-o-up
const ICON_REPORTS: &str = "\u{f024}"; // fa-flag
const ICON_RECENT: &str = "\u{f1da}"; // fa-history
const ICON_PEAK_KILLS: &str = "\u{f05b}"; // fa-crosshairs
const ICON_PEAK_ASSISTS: &str = "\u{f0c0}"; // fa-users
const ICON_PEAK_GPM: &str = "\u{f155}"; // fa-usd
const ICON_PEAK_XPM: &str = "\u{f0e7}"; // fa-bolt
const ICON_PEAK_DURATION: &str = "\u{f017}"; // fa-clock-o
const ICON_PEAK_NETWORTH: &str = "\u{f0d6}"; // fa-money

/// Lays minimap icons for `hero_ids` out side-by-side into one horizontal
/// strip (`hero_icons::HEIGHT` rows tall), one text row per icon pixel-row.
/// A hero with no vendored icon (see `hero_icons.rs`) renders as blank
/// space, so columns stay aligned regardless of which heroes are missing.
fn hero_icon_strip(hero_ids: &[i32]) -> Vec<String> {
    let blank = " ".repeat(hero_icons::WIDTH);
    let icons: Vec<Vec<&str>> = hero_ids
        .iter()
        .map(|&id| match hero_icons::rows(id) {
            Some(rows) => rows.iter().map(String::as_str).collect(),
            None => vec![blank.as_str(); hero_icons::HEIGHT],
        })
        .collect();
    (0..hero_icons::HEIGHT)
        .map(|i| {
            icons
                .iter()
                .map(|rows| rows[i])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// Formats one career-peak line: `<icon> <label>  <value><unit> (<heroes>)`,
/// omitting the hero list entirely when the peak has no record yet.
fn peak_line(icon: &str, label: &str, unit: &str, peak: &crate::stats::PeakRecord) -> String {
    let heroes = if peak.hero_ids.is_empty() {
        String::new()
    } else {
        let names: Vec<String> = peak.hero_ids.iter().map(|&id| heroes::name(id)).collect();
        format!(" ({})", names.join(" / "))
    };
    format!("{icon} {label:<19}{}{unit}{heroes}", peak.value)
}

fn build_sections(pal: &Palette, stats: &Stats) -> Vec<Section> {
    let mut sections = Vec::new();

    let lifetime = &stats.lifetime;
    let lifetime_lines = vec![
        format!("{ICON_MATCHES} Matches: {}", lifetime.total()),
        format!(
            "{ICON_WINS} Wins:    {}{}{} ({:.1}%)",
            pal.green,
            lifetime.wins,
            pal.reset,
            lifetime.winrate()
        ),
        format!(
            "{ICON_LOSSES} Losses:  {}{}{}",
            pal.red, lifetime.losses, pal.reset
        ),
    ];
    sections.push(Section {
        title: "Lifetime".to_string(),
        lines: lifetime_lines,
    });

    if !stats.top_heroes.is_empty() {
        let peaks = &stats.career_peaks;
        sections.push(Section {
            title: "Top Heroes".to_string(),
            lines: stats
                .top_heroes
                .iter()
                .take(5)
                .map(|h| {
                    format!(
                        "{ICON_HERO} {:<16} {}{:>3}W{} {}{:>3}L{} ({:>5.1}%)",
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

        sections.push(Section {
            title: "Career Peaks".to_string(),
            lines: vec![
                peak_line(ICON_PEAK_KILLS, "Most Kills:", "", &peaks.most_kills),
                peak_line(ICON_PEAK_ASSISTS, "Most Assists:", "", &peaks.most_assists),
                peak_line(ICON_PEAK_GPM, "Highest GPM:", "", &peaks.highest_gpm),
                peak_line(ICON_PEAK_XPM, "Highest XPM:", "", &peaks.highest_xpm),
                peak_line(
                    ICON_PEAK_DURATION,
                    "Longest Game:",
                    "s",
                    &peaks.longest_game,
                ),
                peak_line(
                    ICON_PEAK_NETWORTH,
                    "Highest Net Worth:",
                    "",
                    &peaks.highest_net_worth,
                ),
            ],
        });
    }

    if let Some(conduct) = &stats.conduct {
        sections.push(Section {
            title: "Conduct".to_string(),
            lines: vec![
                format!("{ICON_BEHAVIOR} Behavior: {}", conduct.behavior_rating),
                format!(
                    "{ICON_COMMENDS} Commends: {}{}{}",
                    pal.green, conduct.commend_count, pal.reset
                ),
                format!(
                    "{ICON_REPORTS} Reports:  {}{}{}",
                    if conduct.reports_count > 0 {
                        pal.red
                    } else {
                        ""
                    },
                    conduct.reports_count,
                    pal.reset
                ),
                format!(
                    "{ICON_RECENT} Recent matches clean/abandoned: {}/{}",
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
    // Center the (usually shorter) logo vertically against the info panel,
    // rather than pinning it to the top row.
    let logo_offset = rows.saturating_sub(logo.len()) / 2;

    for i in 0..rows {
        let logo_line = i
            .checked_sub(logo_offset)
            .and_then(|j| logo.get(j))
            .map(String::as_str)
            .unwrap_or("");
        let logo_pad = logo_width.saturating_sub(ansi::visible_width(logo_line));
        let right_line = right_lines.get(i).map(String::as_str).unwrap_or("");
        outln!("{logo_line}{}{GAP}{right_line}", " ".repeat(logo_pad));
    }
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

    // Interior width shared by every section's box, so right edges line up
    // the way Omarchy's fastfetch config does. Content lines get a 4-column
    // border+connector prefix ("│ ├ " / "└ └ " / "    "), so budget for that
    // too.
    let width = sections
        .iter()
        .flat_map(|s| {
            std::iter::once(ansi::visible_width(&s.title))
                .chain(s.lines.iter().map(|l| ansi::visible_width(l) + 4))
        })
        .max()
        .unwrap_or(20);
    // +2 for the box's corner columns (┌┐ / └┘), so the header lines up with
    // the box's full outer width, not just its interior.
    let header = format!(
        "{}{}{}",
        pal.bold,
        center(persona_name, width + 2),
        pal.reset
    );

    let top_hero_ids: Vec<i32> = stats.top_heroes.iter().take(5).map(|h| h.hero_id).collect();

    let mut right = vec![header, String::new()];
    for section in &sections {
        if section.title == "Top Heroes" {
            right.extend(hero_icon_strip(&top_hero_ids));
        }
        right.extend(render_section(&pal, section, width));
        right.push(String::new());
    }
    right.pop(); // drop the trailing blank line

    print_with_logo(&right);
}
