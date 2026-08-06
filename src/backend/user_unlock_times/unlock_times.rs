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
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementUnlock {
    pub api_name: String,
    pub display_name: String,
    pub achieved: bool,
    pub unlock_time: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct UnlockStamp {
    pub app_id: u32,
    pub unlock_time: u32,
}

#[derive(Debug, Default)]
pub struct UnlockCache {
    pub stamps: Vec<UnlockStamp>,
    /// Games there was a file for, unlocks or not. One with achievements
    /// missing from here has had its file deleted since Steam last started.
    pub apps: HashSet<u32>,
}

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

pub fn user_stats_file(account_id: u32, app_id: u32) -> Result<PathBuf, SamError> {
    Ok(stats_dir()?.join(format!("UserGameStats_{account_id}_{app_id}.bin")))
}

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

/// Well under the core count: this runs behind a GUI still in use, and what the
/// threads buy is overlapped I/O wait rather than parsing throughput.
const SWEEP_THREADS: usize = 4;

/// Every unlock stamp `account_id`'s files hold, across every app. Blocking.
///
/// Only the per-user files, never the schemas beside them: those hold nothing
/// this needs and are three orders of magnitude larger. How complete they are is
/// not this function's call — Steam writes an app's file when something asks for
/// its stats, once per session, so this is only as complete as the sweep.
///
/// Server-confirmed unlocks only: Steam also invents unlocks locally, flagged
/// dirty, when a game's schema defaults already satisfy an achievement's rule.
pub fn read_all_unlock_stamps(account_id: u32) -> Result<UnlockCache, SamError> {
    let dir = stats_dir()?;
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        eprintln!("[USER UNLOCK TIMES] Failed to list {}: {e}", dir.display());
        SamError::UnknownError
    })?;

    let prefix = format!("UserGameStats_{account_id}_");
    let files: Vec<(u32, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let app_id = name
                .strip_prefix(&prefix)?
                .strip_suffix(".bin")?
                .parse::<u32>()
                .ok()?;
            Some((app_id, entry.path()))
        })
        .collect();

    let per_thread = files.len().div_ceil(SWEEP_THREADS).max(1);
    let apps: HashSet<u32> = files.iter().map(|(app_id, _)| *app_id).collect();
    let mut out = Vec::new();
    std::thread::scope(|scope| {
        let workers: Vec<_> = files
            .chunks(per_thread)
            .map(|chunk| {
                scope.spawn(move || {
                    let mut stamps = Vec::new();
                    for (app_id, path) in chunk {
                        if let Ok(user) = KeyValue::load_as_binary(path) {
                            collect_stamps(&user, *app_id, &mut stamps);
                        }
                    }
                    stamps
                })
            })
            .collect();
        for worker in workers {
            out.extend(worker.join().unwrap_or_default());
        }
    });
    Ok(UnlockCache { stamps: out, apps })
}

fn dirty_mask(group: &KeyValue) -> u32 {
    group
        .children
        .get("dirtybits")
        .map(|d| d.as_i32(0) as u32)
        .unwrap_or(0)
}

fn collect_stamps(user: &KeyValue, app_id: u32, out: &mut Vec<UnlockStamp>) {
    let Some(cache) = find_first(user, "cache") else {
        return;
    };
    for group in cache.children.values() {
        let Some(times) = group.children.get("AchievementTimes") else {
            continue;
        };
        let mask = group
            .children
            .get("data")
            .map(|d| d.as_i32(0) as u32)
            .unwrap_or(0);
        let dirty = dirty_mask(group);
        for (pos_str, value) in &times.children {
            let Ok(pos) = pos_str.parse::<u32>() else {
                continue;
            };
            if pos >= 32 || (mask >> pos) & 1 == 0 || (dirty >> pos) & 1 == 1 {
                continue;
            }
            let unlock_time = value.as_i32(0) as u32;
            if unlock_time == 0 {
                continue;
            }
            out.push(UnlockStamp {
                app_id,
                unlock_time,
            });
        }
    }
}

/// For the API fallback: the names come from one bulk schema parse, so only the
/// unlock times need the Steam API.
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

/// The schema's `bits` subtree maps bit position -> achievement; the matching
/// `cache/<group>` in the user file holds the mask and the stamps.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::key_value::KeyValueData;

    fn node(name: &str, data: KeyValueData) -> KeyValue {
        KeyValue {
            name: name.to_string(),
            data,
            children: std::collections::HashMap::new(),
            valid: true,
        }
    }

    fn group(name: &str, mask: i32, dirty: Option<i32>, times: &[(u32, u32)]) -> KeyValue {
        let mut achievement_times = node("AchievementTimes", KeyValueData::None);
        for &(pos, time) in times {
            achievement_times.children.insert(
                pos.to_string(),
                node(&pos.to_string(), KeyValueData::Int32(time as i32)),
            );
        }

        let mut group = node(name, KeyValueData::None);
        group
            .children
            .insert("data".to_string(), node("data", KeyValueData::Int32(mask)));
        if let Some(dirty) = dirty {
            group.children.insert(
                "dirtybits".to_string(),
                node("dirtybits", KeyValueData::Int32(dirty)),
            );
        }
        group
            .children
            .insert("AchievementTimes".to_string(), achievement_times);
        group
    }

    fn cache(groups: Vec<KeyValue>) -> KeyValue {
        let mut cache = node("cache", KeyValueData::None);
        for group in groups {
            cache.children.insert(group.name.clone(), group);
        }
        let mut root = KeyValue::root();
        root.children.insert("cache".to_string(), cache);
        root
    }

    #[test]
    fn a_clean_group_is_history() {
        let user = cache(vec![group(
            "1",
            0b111,
            None,
            &[(0, 100), (1, 200), (2, 300)],
        )]);
        let mut out = Vec::new();
        collect_stamps(&user, 440, &mut out);

        let mut times: Vec<u32> = out.iter().map(|s| s.unlock_time).collect();
        times.sort_unstable();
        assert_eq!(times, vec![100, 200, 300]);
    }

    #[test]
    fn steams_fiction_is_not_history() {
        let user = cache(vec![group(
            "1",
            0b111,
            Some(0b101),
            &[(0, 100), (1, 200), (2, 300)],
        )]);
        let mut out = Vec::new();
        collect_stamps(&user, 440, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].unlock_time, 200);
    }

    #[test]
    fn a_whole_game_of_fiction_is_dropped() {
        let stamps: Vec<(u32, u32)> = (0..32).map(|pos| (pos, 1_786_568_714)).collect();
        let user = cache(vec![group("1", -1, Some(-1), &stamps)]);
        let mut out = Vec::new();
        collect_stamps(&user, 687_480, &mut out);

        assert!(out.is_empty());
    }
}
