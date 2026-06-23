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

//! Bulk parse of an account's on-disk achievement stats, plus the cache path
//! helpers used to locate the stats files and `localconfig.vdf`.

use crate::backend::key_value::KeyValue;
use crate::utils::ipc_types::SamError;
use crate::utils::steam_locator::SteamLocator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementUnlock {
    pub api_name: String,
    pub display_name: String,
    pub achieved: bool,
    /// Unix seconds; `Some` only when `achieved`.
    pub unlock_time: Option<u32>,
}

/// `<steam root>/appcache/stats`, resolved via the same locator the rest of the
/// app uses (so `SAM_*` overrides and the snap/Flatpak copies stay consistent).
pub fn stats_dir() -> Result<PathBuf, SamError> {
    let schema = SteamLocator::global()
        .read()
        .map_err(|_| SamError::UnknownError)?
        .get_user_game_stats_schema(&0)?;
    schema
        .parent()
        .map(PathBuf::from)
        .ok_or(SamError::UnknownError)
}

/// Path of the cached stats blob for `account_id`/`app_id` (may not exist yet).
pub fn user_stats_file(account_id: u32, app_id: u32) -> Result<PathBuf, SamError> {
    Ok(stats_dir()?.join(format!("UserGameStats_{account_id}_{app_id}.bin")))
}

/// Bulk join: parse the schema and the account's cached stats file once each and
/// return every achievement with its achieved flag and unlock time.
pub fn read_unlock_times(account_id: u32, app_id: u32) -> Result<Vec<AchievementUnlock>, SamError> {
    let dir = stats_dir()?;
    let schema_path = dir.join(format!("UserGameStatsSchema_{app_id}.bin"));
    let user_path = dir.join(format!("UserGameStats_{account_id}_{app_id}.bin"));

    let schema = KeyValue::load_as_binary(&schema_path).map_err(|e| {
        eprintln!(
            "[USER UNLOCK TIMES] Failed to read schema {}: {e}",
            schema_path.display()
        );
        SamError::UnknownError
    })?;
    let user = KeyValue::load_as_binary(&user_path).map_err(|e| {
        eprintln!(
            "[USER UNLOCK TIMES] Failed to read user stats {}: {e}",
            user_path.display()
        );
        SamError::UnknownError
    })?;

    let cache = find_first(&user, "cache");
    let mut out = Vec::new();
    walk(&schema, cache, &mut out);
    Ok(out)
}

/// Read just the achievement list (api name + english display name) from the
/// schema, in schema order. Used for the API fallback: the names come from one
/// bulk schema parse; only the per-user unlock times then need the Steam API.
pub fn read_schema_achievements(app_id: u32) -> Result<Vec<(String, String)>, SamError> {
    let schema_path = stats_dir()?.join(format!("UserGameStatsSchema_{app_id}.bin"));
    let schema = KeyValue::load_as_binary(&schema_path).map_err(|e| {
        eprintln!(
            "[USER UNLOCK TIMES] Failed to read schema {}: {e}",
            schema_path.display()
        );
        SamError::UnknownError
    })?;
    let mut out = Vec::new();
    collect_schema_names(&schema, &mut out);
    Ok(out)
}

fn collect_schema_names(node: &KeyValue, out: &mut Vec<(String, String)>) {
    if let Some(bits) = node.children.get("bits") {
        let mut positioned: Vec<(u32, String, String)> = bits
            .children
            .iter()
            .filter_map(|(pos_str, bit)| {
                let pos = pos_str.parse::<u32>().ok()?;
                let api = bit.children.get("name")?.as_string("");
                let display = bit
                    .children
                    .get("display")
                    .and_then(|d| d.children.get("name"))
                    .and_then(|n| n.children.get("english"))
                    .map(|e| e.as_string(""))
                    .unwrap_or_default();
                Some((pos, api, display))
            })
            .collect();
        positioned.sort_by_key(|(pos, _, _)| *pos);
        for (_, api, display) in positioned {
            out.push((api, display));
        }
    }
    for child in node.children.values() {
        collect_schema_names(child, out);
    }
}

/// Each achievement stat group in the schema has a `bits` subtree mapping bit
/// position -> achievement. The matching `cache/<group>` in the user file holds
/// the achieved bitmask (`data`) and `AchievementTimes/<bit>` unlock stamps.
fn walk(node: &KeyValue, cache: Option<&KeyValue>, out: &mut Vec<AchievementUnlock>) {
    if let Some(bits) = node.children.get("bits") {
        let group = cache.and_then(|c| c.children.get(&node.name));
        let mask = group
            .and_then(|g| g.children.get("data"))
            .map(|d| d.as_i32(0) as u32)
            .unwrap_or(0);
        let times = group.and_then(|g| g.children.get("AchievementTimes"));

        for (pos_str, bit) in &bits.children {
            let Ok(pos) = pos_str.parse::<u32>() else {
                continue;
            };
            let api_name = bit
                .children
                .get("name")
                .map(|n| n.as_string(""))
                .unwrap_or_default();
            let display_name = bit
                .children
                .get("display")
                .and_then(|d| d.children.get("name"))
                .and_then(|n| n.children.get("english"))
                .map(|e| e.as_string(""))
                .unwrap_or_default();
            let achieved = pos < 32 && (mask >> pos) & 1 == 1;
            let unlock_time = times
                .and_then(|t| t.children.get(pos_str))
                .map(|v| v.as_i32(0) as u32)
                .filter(|_| achieved);

            out.push(AchievementUnlock {
                api_name,
                display_name,
                achieved,
                unlock_time,
            });
        }
    }
    for child in node.children.values() {
        walk(child, cache, out);
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
