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

use crate::utils::ipc_types::SamError;
use std::env;
use std::path::{Path, PathBuf};

/// Returns the absolute path of the running executable.
pub fn get_executable_path() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .canonicalize()
        .expect("Failed to canonicalize path")
}

/// Returns a valid directory to store persistent app data (e.g. cache).
pub fn get_app_cache_dir() -> String {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        if let Ok(snap_name) = env::var("SNAP_NAME") {
            return if snap_name == "samrewritten" {
                env::var("SNAP_USER_COMMON").unwrap_or_else(|_| "/tmp".into())
            } else {
                ".".into()
            };
        }

        let folder = PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
            .join(".cache/samrewritten");
        fs::create_dir_all(&folder).expect("Could not create cache folder");
        folder.to_string_lossy().to_string()
    }

    #[cfg(target_os = "windows")]
    {
        env::temp_dir().to_string_lossy().to_string()
    }
}

/// Tries to find the Steam client library path.
pub fn get_steamclient_lib_path() -> Result<String, SamError> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
            return Ok(PathBuf::from(real_home)
                .join("snap/steam/common/.local/share/Steam/linux64/steamclient.so")
                .to_string_lossy()
                .to_string());
        }

        let home = env::var("HOME").map_err(|_| SamError::UnknownError)?;
        let paths = [
            ".steam/debian-installation/linux64/steamclient.so",
            ".steam/sdk64/steamclient.so",
            ".steam/steam/linux64/steamclient.so",
            ".steam/root/linux64/steamclient.so",
            "snap/steam/common/.local/share/Steam/linux64/steamclient.so",
        ];

        for rel in paths {
            let full_path = PathBuf::from(&home).join(rel);
            if full_path.exists() {
                return Ok(full_path.to_string_lossy().to_string());
            }
        }

        Err(SamError::UnknownError)
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::*, RegKey};
        const REG_PATH: &str = "SOFTWARE\\Valve\\Steam";
        const VALUE_NAME: &str = "SteamPath";

        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Ok(subkey) = RegKey::predef(hive).open_subkey(REG_PATH) {
                if let Ok(value) = subkey.get_value::<String, _>(VALUE_NAME) {
                    return Ok(PathBuf::from(value)
                        .join("steamclient64.dll")
                        .to_string_lossy()
                        .to_string());
                }
            }
        }

        Err(SamError::UnknownError)
    }
}

/// Gets the path to the user game stats schema.
pub fn get_user_game_stats_schema_path(app_id: &u32) -> Result<String, SamError> {
    let file_name = format!("UserGameStatsSchema_{app_id}.bin");

    #[cfg(target_os = "linux")]
    {
        if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
            return Ok(PathBuf::from(real_home)
                .join("snap/steam/common/.local/share/Steam/appcache/stats")
                .join(&file_name)
                .to_string_lossy()
                .to_string());
        }

        let home = env::var("HOME").map_err(|_| SamError::UnknownError)?;
        let dirs = [
            ".steam/debian-installation",
            ".steam/steam",
            ".steam/root",
            "snap/steam/common/.local/share/Steam",
        ];

        for dir in dirs {
            let base = PathBuf::from(&home).join(dir);
            if base.exists() {
                return Ok(base
                    .join("appcache/stats")
                    .join(&file_name)
                    .to_string_lossy()
                    .to_string());
            }
        }

        Err(SamError::UnknownError)
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let subkey = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .map_err(|_| SamError::UnknownError)?;

        let path = subkey
            .get_value::<String, _>("SteamPath")
            .map_err(|_| SamError::UnknownError)?;

        Ok(PathBuf::from(path)
            .join("appcache/stats")
            .join(file_name)
            .to_string_lossy()
            .to_string())
    }
}

/// Returns the path to the local app banner image (header.jpg).
pub fn get_local_app_banner_file_path(app_id: &u32) -> Result<String, SamError> {
    let file_path = format!("{app_id}/header.jpg");

    #[cfg(target_os = "linux")]
    {
        if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
            return Ok(PathBuf::from(real_home)
                .join("snap/steam/common/.local/share/Steam/appcache/librarycache")
                .join(&file_path)
                .to_string_lossy()
                .to_string());
        }

        let home = env::var("HOME").map_err(|_| SamError::UnknownError)?;
        let dirs = [
            ".steam/debian-installation",
            ".steam/steam",
            ".steam/root",
            "snap/steam/common/.local/share/Steam",
        ];

        for dir in dirs {
            let base = PathBuf::from(&home).join(dir);
            if base.exists() {
                return Ok(base
                    .join("appcache/librarycache")
                    .join(&file_path)
                    .to_string_lossy()
                    .to_string());
            }
        }

        Err(SamError::UnknownError)
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let subkey = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .map_err(|_| SamError::UnknownError)?;

        let path = subkey
            .get_value::<String, _>("SteamPath")
            .map_err(|_| SamError::UnknownError)?;

        Ok(PathBuf::from(path)
            .join("appcache/librarycache")
            .join(&file_path)
            .to_string_lossy()
            .to_string())
    }
}
