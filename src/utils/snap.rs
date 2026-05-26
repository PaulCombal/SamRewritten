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

//! Snap-specific helpers for the portal-based Steam folder flow.
//!
//! Under strict confinement there is no `personal-files` access to Steam, so the
//! user grants their Steam folder via the XDG FileChooser portal. That mount
//! serves `read()` but refuses the loader's `mmap(PROT_EXEC)`, so `steamclient.so`
//! is mirrored into `$SNAP_USER_COMMON` and loaded from there.

use std::path::{Path, PathBuf};

pub fn is_snap() -> bool {
    std::env::var("SNAP_NAME")
        .map(|n| n == "samrewritten")
        .unwrap_or(false)
}

fn snap_user_common() -> Option<PathBuf> {
    std::env::var("SNAP_USER_COMMON").ok().map(PathBuf::from)
}

fn saved_root_file() -> Option<PathBuf> {
    snap_user_common().map(|d| d.join("steam_root.txt"))
}

/// A previously-picked root, but only if it's still readable — i.e. the portal
/// grant survived.
pub fn load_saved_root() -> Option<PathBuf> {
    let stored = std::fs::read_to_string(saved_root_file()?).ok()?;
    let root = PathBuf::from(stored.trim());
    root.join("linux64/steamclient.so").exists().then_some(root)
}

pub fn save_root(root: &Path) {
    if let Some(file) = saved_root_file() {
        let _ = std::fs::write(file, root.to_string_lossy().as_bytes());
    }
}

/// The frontend reads library-cache banners itself, so it needs the picked root
/// in its own env (the orchestrator child gets it via `spawn_orchestrator`).
pub fn pin_install_root(root: &Path) {
    // SAFETY: called once at GUI startup before any worker threads are spawned.
    unsafe { std::env::set_var("SAM_STEAM_INSTALL_ROOT", root) };
}

/// Forget the saved grant + pinned root so the next start re-prompts. Pair with
/// a re-exec.
pub fn forget_saved_install() {
    if let Some(file) = saved_root_file() {
        let _ = std::fs::remove_file(file);
    }
    // SAFETY: main thread, from a menu action, immediately before re-exec.
    unsafe { std::env::remove_var("SAM_STEAM_INSTALL_ROOT") };
}

/// Mirror `steamclient.so` into `$SNAP_USER_COMMON`: the `fuse.portal` mount the
/// picked folder lives on serves `read()` but refuses `mmap(PROT_EXEC)`, so a
/// dlopen straight from it fails with "failed to map segment from shared object".
pub fn mirror_steamclient(root: &Path) -> std::io::Result<PathBuf> {
    let common =
        snap_user_common().ok_or_else(|| std::io::Error::other("SNAP_USER_COMMON is unset"))?;
    let src = root.join("linux64/steamclient.so");
    let dst = common.join("steamclient.so");
    std::fs::copy(&src, &dst)?;
    Ok(dst)
}

pub fn real_home() -> Option<PathBuf> {
    std::env::var("SNAP_REAL_HOME")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

/// Pre-aim target for the picker. Returned unconditionally — we can't stat it
/// (the `home` interface hides `~/snap`); the unconfined portal opens there.
pub fn snap_steam_default_path() -> Option<PathBuf> {
    real_home().map(|h| h.join("snap/steam/common/.local/share/Steam"))
}
