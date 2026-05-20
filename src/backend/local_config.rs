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

use crate::dev_println;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::ipc_types::SamError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct AppPlaytime {
    pub playtime_minutes: Option<u32>,
    pub last_played: Option<u64>,
}

pub type PlaytimeMap = HashMap<AppId_t, AppPlaytime>;

#[derive(Deserialize)]
struct LocalConfig {
    #[serde(rename = "Software")]
    software: Software,
}

#[derive(Deserialize)]
struct Software {
    #[serde(rename = "Valve", alias = "valve")]
    valve: Valve,
}

#[derive(Deserialize)]
struct Valve {
    #[serde(rename = "Steam", alias = "steam")]
    steam: SteamSection,
}

#[derive(Deserialize)]
struct SteamSection {
    #[serde(rename = "apps", alias = "Apps", default)]
    apps: HashMap<String, AppEntry>,
}

#[derive(Deserialize, Default)]
struct AppEntry {
    // Steam occasionally stores negative playtime values, so accept i32 here
    #[serde(rename = "Playtime", default)]
    playtime: Option<i32>,
    #[serde(rename = "LastPlayed", default)]
    last_played: Option<u64>,
}

pub fn parse_localconfig(path: &Path) -> Result<PlaytimeMap, SamError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        dev_println!(
            "ORCH",
            "Failed to read localconfig.vdf at {}: {e}",
            path.display()
        );
        SamError::UnknownError
    })?;

    let parsed: LocalConfig = keyvalues_serde::from_str(&contents).map_err(|e| {
        dev_println!("ORCH", "Failed to parse localconfig.vdf: {e}");
        SamError::UnknownError
    })?;

    let mut map = PlaytimeMap::new();
    for (key, entry) in parsed.software.valve.steam.apps {
        let playtime_minutes = entry.playtime.filter(|&v| v >= 0).map(|v| v as u32);
        if playtime_minutes.is_none() && entry.last_played.is_none() {
            continue;
        }
        let Ok(app_id) = key.parse::<AppId_t>() else {
            continue;
        };
        map.insert(
            app_id,
            AppPlaytime {
                playtime_minutes,
                last_played: entry.last_played,
            },
        );
    }

    Ok(map)
}
