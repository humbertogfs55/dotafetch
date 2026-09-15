//! Hero id -> minimap icon lookup, embedded at compile time.
//!
//! `data/hero_icons.json` holds a 9x5 truecolor half-block ANSI render (via
//! `chafa`) of each hero's minimap icon, one entry per hero id, generated
//! offline by `scripts/generate_hero_icons.py` from
//! `reference/hero_minimap_icons/` (gitignored, local-only - only this
//! downsampled ANSI derivative is vendored). Like `heroes.rs`, this table
//! drifts behind newly-released heroes until the script is re-run; heroes
//! with no icon simply render no icon (see `render.rs`), the same
//! graceful-fallback spirit as `heroes::name`'s `"Hero <id>"` placeholder.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Visible columns/rows of every icon.
pub const WIDTH: usize = 9;
pub const HEIGHT: usize = 5;

static TABLE: OnceLock<HashMap<u32, Vec<String>>> = OnceLock::new();

fn table() -> &'static HashMap<u32, Vec<String>> {
    TABLE.get_or_init(|| {
        let raw = include_str!("../data/hero_icons.json");
        let parsed: HashMap<String, Vec<String>> =
            serde_json::from_str(raw).expect("data/hero_icons.json is malformed");
        parsed
            .into_iter()
            .filter_map(|(id, rows)| id.parse::<u32>().ok().map(|id| (id, rows)))
            .collect()
    })
}

/// The icon's rows (`HEIGHT` of them, each `WIDTH` visible columns of
/// truecolor half-block ANSI), or `None` if `hero_id` has no vendored icon.
pub fn rows(hero_id: i32) -> Option<&'static [String]> {
    if hero_id < 0 {
        return None;
    }
    table().get(&(hero_id as u32)).map(Vec::as_slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hero_has_icon() {
        let rows = rows(44).expect("Phantom Assassin should have an icon"); // Phantom Assassin
        assert_eq!(rows.len(), HEIGHT);
    }

    #[test]
    fn unknown_hero_has_no_icon() {
        assert!(rows(118).is_none());
        assert!(rows(-1).is_none());
    }
}
