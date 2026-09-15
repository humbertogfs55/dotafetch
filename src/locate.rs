//! Finds the local Steam installation, confirms Dota 2 (app id 570) is
//! installed, and resolves the most-recently-logged-in account's cfg dir.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::vdf;

const DOTA_APP_ID: &str = "570";
const STEAM_ID64_BASE: u64 = 76561197960265728;

#[derive(Debug)]
pub struct LocateError(String);

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for LocateError {}

fn locate_err(msg: impl Into<String>) -> LocateError {
    LocateError(msg.into())
}

pub struct Account {
    pub persona_name: String,
    pub cfg_dir: PathBuf,
    /// Local (non-cloud-synced) `game/dota/cfg` dir inside the Dota 2
    /// install itself, if we could resolve it. Used only for the optional
    /// conduct/behavior summary - never required.
    pub install_cfg_dir: Option<PathBuf>,
}

fn steam_root() -> Result<PathBuf, LocateError> {
    if let Ok(p) = std::env::var("STEAM_ROOT") {
        return Ok(PathBuf::from(p));
    }
    let home = std::env::var("HOME").map_err(|_| locate_err("HOME is not set"))?;
    let candidates = [
        PathBuf::from(&home).join(".steam/root"),
        PathBuf::from(&home).join(".local/share/Steam"),
    ];
    for c in candidates {
        if c.exists() {
            // ~/.steam/root is typically a symlink; canonicalize so
            // downstream joins land on the real directory.
            return fs::canonicalize(&c).or(Ok(c));
        }
    }
    Err(locate_err(
        "could not find a Steam installation (checked $STEAM_ROOT, ~/.steam/root, ~/.local/share/Steam)",
    ))
}

/// Returns the path of the Steam library that has Dota 2 (app 570)
/// installed, if any.
fn library_with_dota(root: &Path) -> Option<PathBuf> {
    let vdf_path = root.join("steamapps/libraryfolders.vdf");
    let src = fs::read_to_string(&vdf_path).ok()?;
    let node = vdf::parse(&src).ok()?;
    let libraries = node.as_block()?;
    libraries.values().find_map(|lib| {
        let has_dota = lib
            .get("apps")
            .and_then(vdf::Node::as_block)
            .is_some_and(|apps| apps.contains_key(DOTA_APP_ID));
        if has_dota {
            lib.get("path")
                .and_then(vdf::Node::as_str)
                .map(PathBuf::from)
        } else {
            None
        }
    })
}

/// Best-effort resolution of the Dota 2 install's local `game/dota/cfg`
/// directory, used only for the optional conduct summary.
fn install_cfg_dir(library_path: &Path) -> Option<PathBuf> {
    let manifest_path = library_path.join("steamapps/appmanifest_570.acf");
    let install_dir = fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|src| vdf::parse(&src).ok())
        .and_then(|node| {
            node.get("installdir")
                .and_then(vdf::Node::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "dota 2 beta".to_string());

    let cfg_dir = library_path
        .join("steamapps/common")
        .join(install_dir)
        .join("game/dota/cfg");
    cfg_dir.exists().then_some(cfg_dir)
}

fn most_recent_user(users: &BTreeMap<String, vdf::Node>) -> Option<(&str, &vdf::Node)> {
    users
        .iter()
        .map(|(id, node)| {
            let ts: i64 = node
                .get("Timestamp")
                .and_then(vdf::Node::as_str)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            (id.as_str(), node, ts)
        })
        .max_by_key(|(_, _, ts)| *ts)
        .map(|(id, node, _)| (id, node))
}

pub fn find_account() -> Result<Account, LocateError> {
    let root = steam_root()?;

    let dota_library = library_with_dota(&root).ok_or_else(|| {
        locate_err("Dota 2 (app 570) is not installed in any Steam library on this machine")
    })?;
    let install_cfg_dir = install_cfg_dir(&dota_library);

    let loginusers_path = root.join("config/loginusers.vdf");
    let loginusers_src = fs::read_to_string(&loginusers_path).map_err(|e| {
        locate_err(format!(
            "could not read {}: {e}",
            loginusers_path.display()
        ))
    })?;
    let loginusers = vdf::parse(&loginusers_src)
        .map_err(|e| locate_err(format!("could not parse {}: {e}", loginusers_path.display())))?;
    let users = loginusers
        .as_block()
        .ok_or_else(|| locate_err("loginusers.vdf: unexpected format"))?;
    let (steam_id64_str, user) = most_recent_user(users)
        .ok_or_else(|| locate_err("loginusers.vdf: no accounts found"))?;
    let steam_id64: u64 = steam_id64_str
        .parse()
        .map_err(|_| locate_err(format!("invalid SteamID64 {steam_id64_str:?}")))?;
    let persona_name = user
        .get("PersonaName")
        .and_then(vdf::Node::as_str)
        .unwrap_or("Unknown")
        .to_string();

    let steam_id32 = steam_id64
        .checked_sub(STEAM_ID64_BASE)
        .ok_or_else(|| locate_err(format!("SteamID64 {steam_id64} is below the SteamID32 base")))?;

    let cfg_dir = root
        .join("userdata")
        .join(steam_id32.to_string())
        .join(DOTA_APP_ID)
        .join("remote/cfg");

    if !cfg_dir.join("stats.dat").exists() {
        return Err(locate_err(format!(
            "no Dota 2 stats found at {} - run Dota 2 at least once so it can sync local stats",
            cfg_dir.display()
        )));
    }

    Ok(Account {
        persona_name,
        cfg_dir,
        install_cfg_dir,
    })
}
