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
use std::collections::HashSet;
use std::path::PathBuf;

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
                && let Some(id) = rest.strip_suffix(".bin").and_then(|s| s.parse::<u32>().ok())
            {
                schemas_present.insert(id);
                continue;
            }
            if let Some(rest) = name.strip_prefix(&user_prefix)
                && let Some(id) = rest.strip_suffix(".bin").and_then(|s| s.parse::<u32>().ok())
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

        let schema_path = self.stats_dir.join(format!("UserGameStatsSchema_{app_id}.bin"));
        let user_path = self
            .stats_dir
            .join(format!("UserGameStats_{}_{app_id}.bin", self.account_id));

        let schema = KeyValue::load_as_binary(&schema_path).ok()?;
        let user_stats = KeyValue::load_as_binary(&user_path).ok()?;

        let (total, unlocked) = count_pair(&schema, &user_stats, app_id);
        if total == 0 || unlocked > total {
            return None;
        }
        Some((total, unlocked))
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

/// Walks the schema for any node with a `bits` subtree, popcounting the
/// matching `cache/<stat_idx>/data` integer from the user-stats file.
fn count_pair(schema: &KeyValue, user_stats: &KeyValue, app_id: u32) -> (u32, u32) {
    let cache = find_first(user_stats, "cache");
    let mut total: u32 = 0;
    let mut unlocked: u32 = 0;
    walk(schema, cache, &mut total, &mut unlocked, app_id);
    (total, unlocked)
}

fn walk(
    node: &KeyValue,
    cache: Option<&KeyValue>,
    total: &mut u32,
    unlocked: &mut u32,
    app_id: u32,
) {
    if let Some(bits) = node.children.get("bits") {
        let positions: Vec<u32> = bits
            .children
            .keys()
            .filter_map(|k| k.parse::<u32>().ok())
            .collect();
        *total += positions.len() as u32;

        // Single i32 `data` slot per stat group caps achievements at 32 bits.
        // If Steam ever ships a stat with >32 bits we'd silently undercount.
        if positions.iter().any(|p| *p >= 32) {
            eprintln!(
                "[LOCAL_STATS] app {app_id} stat {} has bit position >= 32; counts may be undercounted",
                node.name
            );
        }

        if let Some(cache) = cache {
            let mask = cache
                .children
                .get(&node.name)
                .and_then(|s| s.children.get("data"))
                .map(|d| d.as_i32(0) as u32)
                .unwrap_or(0);
            for pos in &positions {
                if *pos < 32 && (mask >> *pos) & 1 == 1 {
                    *unlocked += 1;
                }
            }
        }
    }
    for child in node.children.values() {
        walk(child, cache, total, unlocked, app_id);
    }
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
