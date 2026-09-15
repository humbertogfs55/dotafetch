//! Hero id -> localized name lookup, embedded at compile time.
//!
//! This is public, effectively-static Dota reference data (vendored from
//! https://github.com/odota/dotaconstants, `build/heroes.json`), not account
//! data - it never touches the network at runtime.

use std::collections::HashMap;
use std::sync::OnceLock;

static TABLE: OnceLock<HashMap<u32, String>> = OnceLock::new();

fn table() -> &'static HashMap<u32, String> {
    TABLE.get_or_init(|| {
        let raw = include_str!("../data/heroes.json");
        let parsed: HashMap<String, String> =
            serde_json::from_str(raw).expect("data/heroes.json is malformed");
        parsed
            .into_iter()
            .filter_map(|(id, name)| id.parse::<u32>().ok().map(|id| (id, name)))
            .collect()
    })
}

pub fn name(hero_id: i32) -> String {
    if hero_id < 0 {
        return format!("Hero {hero_id}");
    }
    table()
        .get(&(hero_id as u32))
        .cloned()
        .unwrap_or_else(|| format!("Hero {hero_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_heroes_resolve() {
        assert_eq!(name(1), "Anti-Mage");
        assert_eq!(name(8), "Juggernaut");
    }

    #[test]
    fn unknown_hero_falls_back() {
        assert_eq!(name(118), "Hero 118");
        assert_eq!(name(99999), "Hero 99999");
    }
}
