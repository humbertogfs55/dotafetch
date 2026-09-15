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

#[derive(Debug)]
pub struct Stats {
    pub lifetime: Lifetime,
    pub top_heroes: Vec<HeroRecord>,
    pub conduct: Option<Conduct>,
}

fn field_i32(dict: &Value, key: &str) -> i32 {
    dict.get(key).and_then(Value::as_i32).unwrap_or(0)
}

fn read_vbkv(path: &Path) -> Result<Value, StatsError> {
    let bytes = fs::read(path).map_err(|e| stats_err(format!("could not read {}: {e}", path.display())))?;
    vbkv::parse(&bytes).map_err(|e| stats_err(format!("could not parse {}: {e}", path.display())))
}

fn build_lifetime_and_heroes(root: &Value) -> Result<(Lifetime, Vec<HeroRecord>), StatsError> {
    let standings = root
        .get("Stats")
        .and_then(|s| s.get("hero_standings"))
        .and_then(|s| s.get("standings"))
        .and_then(Value::as_dict)
        .ok_or_else(|| stats_err("stats.dat: missing Stats.hero_standings.standings"))?;

    let mut lifetime = Lifetime::default();
    let mut heroes = Vec::with_capacity(standings.len());

    for entry in standings.values() {
        let hero_id = field_i32(entry, "hero_id");
        let wins = field_i32(entry, "wins");
        let losses = field_i32(entry, "losses");
        lifetime.wins += wins as i64;
        lifetime.losses += losses as i64;
        // hero_id 0 is a placeholder bucket in the local cache, not a real
        // hero - keep it in the lifetime total but exclude it from the
        // per-hero leaderboard.
        if hero_id != 0 {
            heroes.push(HeroRecord {
                hero_id,
                wins,
                losses,
            });
        }
    }

    heroes.sort_by_key(|h| std::cmp::Reverse(h.games()));
    Ok((lifetime, heroes))
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
        behavior_rating: fields.get("behavior_rating").cloned().unwrap_or_default(),
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

    let (lifetime, top_heroes) = build_lifetime_and_heroes(&root)?;
    let conduct = read_conduct(install_cfg_dir);

    Ok(Stats {
        lifetime,
        top_heroes,
        conduct,
    })
}
