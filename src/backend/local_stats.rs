// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Local-disk fast path for achievement counts. Reads Steam's
//! `appcache/stats/UserGameStatsSchema_<appid>.bin` and
//! `appcache/stats/UserGameStats_<account_id>_<appid>.bin` to avoid the
//! IPC stats round-trip. Misses (e.g. CS:GO's stub schema) fall back to IPC.

use crate::backend::key_value::KeyValue;
use crate::utils::steam_locator::SteamLocator;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

pub struct LocalIndex {
    stats_dir: PathBuf,
    account_id: u32,
    schemas_present: HashSet<u32>,
    user_stats_present: HashSet<u32>,
}

impl LocalIndex {
    pub fn build(account_id: u32) -> Option<Self> {
        let stats_dir = locate_stats_dir()?;
        let entries = std::fs::read_dir(&stats_dir).ok()?;

        let mut schemas_present: HashSet<u32> = HashSet::new();
        let mut user_stats_present: HashSet<u32> = HashSet::new();
        let user_prefix = format!("UserGameStats_{account_id}_");

        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if let Some(rest) = name.strip_prefix("UserGameStatsSchema_")
                && let Some(id) = rest
                    .strip_suffix(".bin")
                    .and_then(|s| s.parse::<u32>().ok())
            {
                schemas_present.insert(id);
                continue;
            }
            if let Some(rest) = name.strip_prefix(&user_prefix)
                && let Some(id) = rest
                    .strip_suffix(".bin")
                    .and_then(|s| s.parse::<u32>().ok())
            {
                user_stats_present.insert(id);
            }
        }

        Some(Self {
            stats_dir,
            account_id,
            schemas_present,
            user_stats_present,
        })
    }

    /// `None` if either file is missing, unparseable, or the result looks
    /// untrustworthy (zero total or unlocked > total — e.g. CS:GO stub).
    pub fn try_read(&self, app_id: u32) -> Option<(u32, u32)> {
        if !self.schemas_present.contains(&app_id) || !self.user_stats_present.contains(&app_id) {
            return None;
        }
        self.read_one(app_id)
    }

    pub fn read_all(&self) -> HashMap<u32, (u32, u32)> {
        let ids: Vec<u32> = self
            .schemas_present
            .intersection(&self.user_stats_present)
            .copied()
            .collect();
        let per_thread = ids.len().div_ceil(SWEEP_THREADS).max(1);
        let mut out = HashMap::with_capacity(ids.len());
        std::thread::scope(|scope| {
            let workers: Vec<_> = ids
                .chunks(per_thread)
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .filter_map(|&app_id| Some((app_id, self.read_one(app_id)?)))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for worker in workers {
                match worker.join() {
                    Ok(read) => out.extend(read),
                    Err(e) => eprintln!("[LOCAL_STATS] Sweep worker panicked: {e:?}"),
                }
            }
        });
        out
    }

    fn read_one(&self, app_id: u32) -> Option<(u32, u32)> {
        let schema_path = self
            .stats_dir
            .join(format!("UserGameStatsSchema_{app_id}.bin"));
        let user_path = self
            .stats_dir
            .join(format!("UserGameStats_{}_{app_id}.bin", self.account_id));

        let bits = cached_schema_bits(app_id, &schema_path)?;
        let user_stats = KeyValue::load_as_binary(&user_path).ok()?;

        let (total, unlocked) = count_from_bits(&bits, &user_stats);
        (total != 0 && unlocked <= total).then_some((total, unlocked))
    }
}

const SWEEP_THREADS: usize = 4;

/// Only the bit layout is kept: schemas dwarf the user-stats files beside them
/// (47 MB against 3.3 MB here) and change only when Steam re-downloads them.
fn cached_schema_bits(app_id: u32, schema_path: &Path) -> Option<Arc<SchemaBits>> {
    type BitsCache = Mutex<HashMap<u32, (Option<SystemTime>, Arc<SchemaBits>)>>;
    static CACHE: LazyLock<BitsCache> = LazyLock::new(|| Mutex::new(HashMap::new()));

    let mtime = std::fs::metadata(schema_path)
        .and_then(|m| m.modified())
        .ok();
    if let Some((cached_mtime, bits)) = CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&app_id)
        && *cached_mtime == mtime
    {
        return Some(Arc::clone(bits));
    }

    let schema = KeyValue::load_as_binary(schema_path).ok()?;
    let bits = Arc::new(schema_bits(&schema, app_id));
    CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(app_id, (mtime, Arc::clone(&bits)));
    Some(bits)
}

/// Empty when the schema is missing or unparseable, leaving only the game default.
///
/// Memoised on the schema's mtime: the parse is ~30 ms on the largest schema seen
/// and runs on every app open and refresh, while the answer only changes when
/// Steam re-downloads the schema.
pub fn read_schema_languages(app_id: u32) -> Vec<String> {
    /// `app_id -> (schema mtime it was read at, languages)`.
    type LanguageCache = Mutex<HashMap<u32, (Option<SystemTime>, Vec<String>)>>;
    static CACHE: LazyLock<LanguageCache> = LazyLock::new(|| Mutex::new(HashMap::new()));

    let Some(stats_dir) = locate_stats_dir() else {
        return vec![];
    };
    let schema_path = stats_dir.join(format!("UserGameStatsSchema_{app_id}.bin"));
    let mtime = std::fs::metadata(&schema_path)
        .and_then(|m| m.modified())
        .ok();

    if let Some((cached_mtime, languages)) = CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&app_id)
        && *cached_mtime == mtime
    {
        return languages.clone();
    }

    // Outside the lock: a duplicated concurrent parse is cheaper than holding
    // every other app id up for the length of one.
    let languages = KeyValue::load_as_binary(&schema_path)
        .map(|schema| schema_languages(&schema))
        .unwrap_or_default();
    CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(app_id, (mtime, languages.clone()));
    languages
}

/// Shared with the child, which resolves the picked language against the tree it
/// has already loaded.
pub fn schema_languages(schema: &KeyValue) -> Vec<String> {
    let mut found: HashSet<String> = HashSet::new();
    collect_languages(schema, &mut found);
    // Steam's internal placeholder pseudo-language, never a real translation.
    found.retain(|l| !l.eq_ignore_ascii_case("token"));

    let mut languages: Vec<String> = found.into_iter().collect();
    languages.sort();
    languages
}

/// `display/name` and `display/desc` hold one child per shipped language.
fn collect_languages(node: &KeyValue, out: &mut HashSet<String>) {
    if node.name == "display" {
        for field in ["name", "desc"] {
            if let Some(kv) = node.children.get(field) {
                out.extend(kv.children.keys().cloned());
            }
        }
    }
    for child in node.children.values() {
        collect_languages(child, out);
    }
}

fn locate_stats_dir() -> Option<PathBuf> {
    let sample = SteamLocator::global()
        .read()
        .ok()?
        .get_user_game_stats_schema(&0)
        .ok()?;
    sample.parent().map(PathBuf::from)
}

type SchemaBits = Vec<(String, Vec<u32>)>;

fn schema_bits(schema: &KeyValue, app_id: u32) -> SchemaBits {
    let mut out = SchemaBits::new();
    walk(schema, app_id, &mut out);
    out
}

fn walk(node: &KeyValue, app_id: u32, out: &mut SchemaBits) {
    if let Some(bits) = node.children.get("bits") {
        let positions: Vec<u32> = bits
            .children
            .keys()
            .filter_map(|k| k.parse::<u32>().ok())
            .collect();

        // Single i32 `data` slot per stat group caps achievements at 32 bits.
        // If Steam ever ships a stat with >32 bits we'd silently undercount.
        if positions.iter().any(|p| *p >= 32) {
            eprintln!(
                "[LOCAL_STATS] app {app_id} stat {} has bit position >= 32; counts may be undercounted",
                node.name
            );
        }

        out.push((node.name.clone(), positions));
    }
    for child in node.children.values() {
        walk(child, app_id, out);
    }
}

fn count_from_bits(bits: &SchemaBits, user_stats: &KeyValue) -> (u32, u32) {
    let cache = find_first(user_stats, "cache");
    let mut total: u32 = 0;
    let mut unlocked: u32 = 0;
    for (group, positions) in bits {
        total += positions.len() as u32;
        let Some(cache) = cache else {
            continue;
        };
        let mask = cache
            .children
            .get(group)
            .and_then(|s| s.children.get("data"))
            .map(|d| d.as_i32(0) as u32)
            .unwrap_or(0);
        for pos in positions {
            if *pos < 32 && (mask >> *pos) & 1 == 1 {
                unlocked += 1;
            }
        }
    }
    (total, unlocked)
}

fn find_first<'a>(node: &'a KeyValue, name: &str) -> Option<&'a KeyValue> {
    if node.name == name {
        return Some(node);
    }
    for child in node.children.values() {
        if let Some(hit) = find_first(child, name) {
            return Some(hit);
        }
    }
    None
}
