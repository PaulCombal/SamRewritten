// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
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

use std::env;
use std::path::PathBuf;

pub fn get_executable_path() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .canonicalize() // Resolves symlinks to absolute path
        .expect("Failed to canonicalize path")
}

/// This function returns a valid directory where app data can be stored for a longer period of time.
#[inline]
#[cfg(target_os = "linux")]
pub fn get_app_cache_dir() -> String {
    use std::fs;
    if let Ok(snap_name) = env::var("SNAP_NAME") {
        if snap_name == "samrewritten" {
            return env::var("SNAP_USER_COMMON").unwrap_or(String::from("/tmp"));
        }

        // Most likely a dev config
        return ".".to_owned();
    }

    // Non-snap release
    let folder = env::var("HOME").unwrap_or("/tmp".to_owned()) + "/.cache/samrewritten";
    fs::create_dir_all(&folder).expect("Could not create temp folder");
    folder
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_app_cache_dir() -> String {
    std::env::temp_dir()
        .to_str()
        .expect("Failed to convert temp dir to string")
        .to_owned()
}

#[inline]
#[cfg(target_os = "linux")]
pub fn get_steamclient_lib_path() -> String {
    if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
        return real_home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
    }

    if let Ok(home) = env::var("HOME") {
        return home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
    }

    panic!("Failed to get Steam client library path");
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_steamclient_lib_path() -> String {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Valve\\Steam")
        .expect("Failed to open Steam registry key")
        .get_value::<String, &'static str>("SteamPath")
        .expect("Failed to get Steam install path from registry")
        + "\\steamclient64.dll"
}

#[inline]
#[cfg(target_os = "linux")]
pub fn get_user_game_stats_schema_path(app_id: &u32) -> String {
    if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
        return real_home
            + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_"
            + &app_id.to_string()
            + ".bin";
    }

    if let Ok(home) = env::var("HOME") {
        return home
            + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_"
            + &app_id.to_string()
            + ".bin";
    }

    panic!("Failed to get User Game Stats Schema path");
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_user_game_stats_schema_path(app_id: &u32) -> String {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Valve\\Steam")
        .expect("Failed to open Steam registry key")
        .get_value::<String, &'static str>("SteamPath")
        .expect("Failed to get Steam install path from registry")
        + &format!("/appcache/stats/UserGameStatsSchema_{app_id}.bin")
}

#[inline]
#[cfg(target_os = "linux")]
pub fn get_local_app_banner_file_path(app_id: &u32) -> String {
    if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
        return real_home
            + "/snap/steam/common/.local/share/Steam/appcache/librarycache/"
            + &app_id.to_string()
            + "/header.jpg";
    }

    if let Ok(home) = env::var("HOME") {
        return home
            + "/snap/steam/common/.local/share/Steam/appcache/librarycache/"
            + &app_id.to_string()
            + "/header.jpg";
    }

    panic!("Failed to get User Game Stats Schema path");
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_local_app_banner_file_path(app_id: &u32) -> String {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("SOFTWARE\\Valve\\Steam")
        .expect("Failed to open Steam registry key")
        .get_value::<String, &'static str>("SteamPath")
        .expect("Failed to get Steam install path from registry")
        + &format!("\\appcache\\librarycache\\{app_id}\\main_header.jpg")
}
