//! Builds typed stats from the parsed local Dota 2 files.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::vbkv::{self, Value};

#[derive(Debug)]
pub struct StatsError(String);

impl fmt::Display for StatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for StatsError {}

fn stats_err(msg: impl Into<String>) -> StatsError {
    StatsError(msg.into())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Lifetime {
    pub wins: i64,
    pub losses: i64,
}

impl Lifetime {
    pub fn total(&self) -> i64 {
        self.wins + self.losses
    }

    pub fn winrate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            self.wins as f64 / self.total() as f64 * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeroRecord {
    pub hero_id: i32,
    pub wins: i32,
    pub losses: i32,
}

impl HeroRecord {
    pub fn games(&self) -> i32 {
        self.wins + self.losses
    }

    pub fn winrate(&self) -> f64 {
        if self.games() == 0 {
            0.0
        } else {
            self.wins as f64 / self.games() as f64 * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct Conduct {
    pub behavior_rating: String,
    pub commend_count: i32,
    pub reports_count: i32,
    pub matches_clean: i32,
    pub matches_abandoned: i32,
}

/// A single-game career-best value, and every hero that reached it (local
/// per-hero peaks can tie, e.g. two heroes sharing a best-assists record).
#[derive(Debug, Clone, Default)]
pub struct PeakRecord {
    pub value: i32,
    pub hero_ids: Vec<i32>,
}

impl PeakRecord {
    fn consider(&mut self, value: i32, hero_id: i32) {
        if value > self.value {
            self.value = value;
            self.hero_ids = vec![hero_id];
        } else if value == self.value && value > 0 {
            self.hero_ids.push(hero_id);
        }
    }
}

/// Career-best single-game records, each drawn from the same per-hero
/// standings entries as `top_heroes` (`best_kills`, `best_assists`, etc.).
#[derive(Debug, Clone, Default)]
pub struct CareerPeaks {
    pub most_kills: PeakRecord,
    pub most_assists: PeakRecord,
    pub highest_gpm: PeakRecord,
    pub highest_xpm: PeakRecord,
    pub longest_game: PeakRecord,
    pub highest_net_worth: PeakRecord,
}

#[derive(Debug)]
pub struct Stats {
    pub lifetime: Lifetime,
    pub top_heroes: Vec<HeroRecord>,
    pub career_peaks: CareerPeaks,
    pub conduct: Option<Conduct>,
}

fn field_i32(dict: &Value, key: &str) -> i32 {
    dict.get(key).and_then(Value::as_i32).unwrap_or(0)
}

/// Maps Dota's `EBehaviorRating` values to a short display label. The local
/// conduct file writes the enum's name (`k_eBehaviorGood`), but a bare
/// numeric ID (`0`) covers a future or differently-formatted conduct file.
/// Anything unrecognized falls back to the raw value rather than hiding it.
fn behavior_label(raw: &str) -> String {
    match raw {
        "k_eBehaviorGood" | "0" => "Good".to_string(),
        "k_eBehaviorWarning" | "1" => "Warning".to_string(),
        "k_eBehaviorBad" | "2" => "Bad".to_string(),
        other => other.to_string(),
    }
}

fn read_vbkv(path: &Path) -> Result<Value, StatsError> {
    let bytes = fs::read(path).map_err(|e| stats_err(format!("could not read {}: {e}", path.display())))?;
    vbkv::parse(&bytes).map_err(|e| stats_err(format!("could not parse {}: {e}", path.display())))
}

fn build_lifetime_heroes_and_peaks(
    root: &Value,
) -> Result<(Lifetime, Vec<HeroRecord>, CareerPeaks), StatsError> {
    let standings = root
        .get("Stats")
        .and_then(|s| s.get("hero_standings"))
        .and_then(|s| s.get("standings"))
        .and_then(Value::as_dict)
        .ok_or_else(|| stats_err("stats.dat: missing Stats.hero_standings.standings"))?;

    let mut lifetime = Lifetime::default();
    let mut heroes = Vec::with_capacity(standings.len());
    let mut peaks = CareerPeaks::default();

    for entry in standings.values() {
        let hero_id = field_i32(entry, "hero_id");
        let wins = field_i32(entry, "wins");
        let losses = field_i32(entry, "losses");
        lifetime.wins += wins as i64;
        lifetime.losses += losses as i64;
        // hero_id 0 is a placeholder bucket in the local cache, not a real
        // hero - keep it in the lifetime total but exclude it from the
        // per-hero leaderboard and the career-peak records.
        if hero_id != 0 {
            heroes.push(HeroRecord {
                hero_id,
                wins,
                losses,
            });

            peaks
                .most_kills
                .consider(field_i32(entry, "best_kills"), hero_id);
            peaks
                .most_assists
                .consider(field_i32(entry, "best_assists"), hero_id);
            peaks
                .highest_gpm
                .consider(field_i32(entry, "best_gpm"), hero_id);
            peaks
                .highest_xpm
                .consider(field_i32(entry, "best_xpm"), hero_id);
            peaks
                .longest_game
                .consider(field_i32(entry, "longest_game_peak"), hero_id);
            peaks
                .highest_net_worth
                .consider(field_i32(entry, "networth_peak"), hero_id);
        }
    }

    heroes.sort_by_key(|h| std::cmp::Reverse(h.games()));
    Ok((lifetime, heroes, peaks))
}

/// Parses the plain-text `latest_conduct_*.txt` file: a single line of
/// whitespace-separated `key: value` pairs. Optional - the file lives
/// alongside the game install, not in the cloud-synced userdata dir, so a
/// missing file (or install we couldn't locate) just means no conduct box.
fn read_conduct(install_cfg_dir: Option<&Path>) -> Option<Conduct> {
    let dir = install_cfg_dir?;
    let entry = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("latest_conduct_")
        })
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())?;
    let text = fs::read_to_string(entry.path()).ok()?;

    let mut fields = std::collections::HashMap::new();
    let mut tokens = text.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        if let Some(key) = tok.strip_suffix(':') {
            if let Some(value) = tokens.next() {
                fields.insert(key.to_string(), value.to_string());
            }
        }
    }

    Some(Conduct {
        behavior_rating: fields
            .get("behavior_rating")
            .map(|s| behavior_label(s))
            .unwrap_or_default(),
        commend_count: fields
            .get("commend_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        reports_count: fields
            .get("reports_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        matches_clean: fields
            .get("matches_clean")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        matches_abandoned: fields
            .get("matches_abandoned")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    })
}

pub fn build(cfg_dir: &Path, install_cfg_dir: Option<&Path>) -> Result<Stats, StatsError> {
    let stats_path = cfg_dir.join("stats.dat");
    let root = read_vbkv(&stats_path)?;

    let (lifetime, top_heroes, career_peaks) = build_lifetime_heroes_and_peaks(&root)?;
    let conduct = read_conduct(install_cfg_dir);

    Ok(Stats {
        lifetime,
        top_heroes,
        career_peaks,
        conduct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_label_maps_known_values() {
        assert_eq!(behavior_label("k_eBehaviorGood"), "Good");
        assert_eq!(behavior_label("k_eBehaviorWarning"), "Warning");
        assert_eq!(behavior_label("k_eBehaviorBad"), "Bad");
        assert_eq!(behavior_label("0"), "Good");
        assert_eq!(behavior_label("1"), "Warning");
        assert_eq!(behavior_label("2"), "Bad");
    }

    #[test]
    fn behavior_label_falls_back_to_raw_value() {
        assert_eq!(behavior_label("k_eBehaviorSomethingNew"), "k_eBehaviorSomethingNew");
    }
}
