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
