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

/// User override for the persistent cache location, checked before any platform default.
const CACHE_DIR_ENV: &str = "SAM_CACHE_DIR";

/// This function returns a valid directory where app data can be stored for a longer period of time.
#[inline]
#[cfg(target_os = "linux")]
pub fn get_app_cache_dir() -> PathBuf {
    use std::fs;

    // Checked before CACHE_DIR_ENV: confinement makes an arbitrary path unwritable anyway.
    if let Ok(snap_name) = env::var("SNAP_NAME") {
        if snap_name == "samrewritten" {
            return env::var_os("SNAP_USER_COMMON")
                .map(PathBuf::from)
                .unwrap_or(PathBuf::from("/tmp"));
        }

        // Most likely a dev config
        return PathBuf::from(".");
    }

    if let Some(folder) = env::var_os(CACHE_DIR_ENV).filter(|dir| !dir.is_empty()) {
        let folder = PathBuf::from(folder);
        match fs::create_dir_all(&folder) {
            Ok(()) => return folder,
            Err(e) => eprintln!(
                "Could not create {CACHE_DIR_ENV} folder {}: {e}",
                folder.display()
            ),
        }
    }

    let folder = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from)
                .unwrap_or(PathBuf::from("/tmp"))
                .join(".cache")
        })
        .join("samrewritten");
    if let Err(e) = fs::create_dir_all(&folder) {
        eprintln!("Could not create cache folder {}: {e}", folder.display());
    }
    folder
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_app_cache_dir() -> PathBuf {
    if let Some(folder) = env::var_os(CACHE_DIR_ENV).filter(|dir| !dir.is_empty()) {
        let folder = PathBuf::from(folder);
        match std::fs::create_dir_all(&folder) {
            Ok(()) => return folder,
            Err(e) => eprintln!(
                "Could not create {CACHE_DIR_ENV} folder {}: {e}",
                folder.display()
            ),
        }
    }

    let folder = env::var_os("LOCALAPPDATA")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("samrewritten");
    if let Err(e) = std::fs::create_dir_all(&folder) {
        eprintln!("Could not create cache folder {}: {e}", folder.display());
    }
    folder
}

// Ensure <temp>/samrewritten-<uid>
// The dir name is resolved once; create_dir_all still runs per call so it self-heals
#[cfg(feature = "gui")]
pub fn get_temp_cache_dir() -> &'static std::path::Path {
    static DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
        // /tmp is shared and sticky, so a plain `samrewritten` would be owned by whoever
        // launched first and unwritable for every other user on the machine.
        #[cfg(target_os = "linux")]
        let name = {
            use std::os::unix::fs::MetadataExt;
            match std::fs::metadata("/proc/self") {
                Ok(m) => format!("samrewritten-{}", m.uid()),
                Err(_) => String::from("samrewritten"),
            }
        };
        // %TEMP% is already per-user
        #[cfg(not(target_os = "linux"))]
        let name = String::from("samrewritten");

        env::temp_dir().join(name)
    });

    let _ = std::fs::create_dir_all(&*DIR);
    &DIR
}
