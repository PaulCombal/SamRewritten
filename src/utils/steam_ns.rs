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

//! Flatpak Steam support, Linux only.
//!
//! Flatpak runs the Steam client in its own PID namespace. Steam's IPC tracks
//! each connection's liveness by PID, so a process on the host (whose PID is
//! meaningless inside Steam's namespace) gets its cross-process pipe reaped
//! mid-call — the "broken pipe" failure. The fix is to put every process that
//! loads `steamclient.so` into Steam's PID namespace.
//!
//! The Flatpak's PID namespace is owned by an unprivileged user namespace our
//! own uid created, so we can join it without root: `setns` into the user
//! namespace (granting CAP_SYS_ADMIN there), then the PID namespace, then
//! `fork` (a PID-namespace `setns` only takes effect for children). We keep the
//! host mount namespace, so our binary and the Flatpak `steamclient.so` stay
//! reachable, and the network namespace is already shared (Steam's IPC is
//! loopback TCP).
//!
//! Only the orchestrator calls this, at startup before any threads exist; the
//! app-server children it spawns inherit the namespace.

use crate::dev_println;
use crate::utils::steam_locator::SteamLocator;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

const CLONE_NEWUSER: c_int = 0x1000_0000;
const CLONE_NEWPID: c_int = 0x2000_0000;

unsafe extern "C" {
    fn setns(fd: c_int, nstype: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

enum NsOutcome {
    Entered,
    ParentExit(u8),
    /// e.g. a confined build denied `setns`.
    Unavailable,
}

/// The locator is the single source of truth for which install we use (it lists
/// Flatpak first); this only reacts to that choice. `Some(code)` means we are
/// the post-fork parent stub and the caller must exit; `None` means carry on as
/// the orchestrator.
pub fn enter_flatpak_steam_ns_if_needed() -> Option<u8> {
    let lib = SteamLocator::get_steamclient_lib_path(true)?;
    if !lib.to_string_lossy().contains("com.valvesoftware.Steam") {
        return None;
    }

    let Some(pid) = detect_flatpak_steam() else {
        eprintln!(
            "[STEAM NS] Flatpak Steam is the selected install but no running Flatpak Steam \
             process was found; the connection will likely fail"
        );
        return None;
    };

    dev_println!("STEAM NS", "Joining Flatpak Steam namespace via pid {pid}");
    match enter_namespace(pid) {
        NsOutcome::Entered => None,
        NsOutcome::ParentExit(code) => Some(code),
        NsOutcome::Unavailable => {
            eprintln!(
                "[STEAM NS] Could not join Flatpak Steam's namespace (is this a confined build?); \
                 Steam integration may be unstable"
            );
            None
        }
    }
}

/// Install roots a Steam client is currently running from, read from the
/// `steamclient.so` each `steam` process has mapped (host-visible for native,
/// Flatpak and Snap alike).
pub fn running_steam_install_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    let Ok(entries) = fs::read_dir("/proc") else {
        return roots;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };

        if fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_default()
            .trim()
            != "steam"
        {
            continue;
        }

        if let Some(root) = steam_root_from_maps(pid)
            && !roots.contains(&root)
        {
            roots.push(root);
        }
    }

    roots
}

// `steamclient.so` lives at `<root>/<arch>/steamclient.so`, so the root is two
// levels up from the path the process has mapped.
fn steam_root_from_maps(pid: i32) -> Option<PathBuf> {
    let maps = fs::read_to_string(format!("/proc/{pid}/maps")).ok()?;
    for line in maps.lines() {
        if !line.contains("steamclient.so") {
            continue;
        }
        let path = Path::new(&line[line.find('/')?..]);
        if path.file_name()?.to_str()? != "steamclient.so" {
            continue;
        }
        return fs::canonicalize(path.parent()?.parent()?).ok();
    }
    None
}

/// Whether Steam is currently running from the install we'd load. Connecting to
/// any other install half-succeeds over Steam's shared loopback IPC and then
/// crashes on the first app-manager call, so this is the check that keeps us on
/// the clean `SteamConnectionFailed` path instead.
pub fn loaded_install_is_running() -> bool {
    let Some(target) = SteamLocator::get_steamclient_lib_path(true)
        .and_then(|lib| lib.parent()?.parent().map(Path::to_path_buf))
        .and_then(|root| fs::canonicalize(root).ok())
    else {
        return false;
    };

    running_steam_install_roots().contains(&target)
}

/// A match is a process named `steam`, in a different PID namespace than us,
/// whose command line points into the Flatpak Steam install.
fn detect_flatpak_steam() -> Option<i32> {
    let self_pidns = fs::read_link("/proc/self/ns/pid").ok()?;

    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };

        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        if comm.trim() != "steam" {
            continue;
        }

        match fs::read_link(format!("/proc/{pid}/ns/pid")) {
            Ok(ns) if ns == self_pidns => continue,
            Err(_) => continue,
            Ok(_) => {}
        }

        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if String::from_utf8_lossy(&cmdline).contains("com.valvesoftware.Steam") {
            return Some(pid);
        }
    }

    None
}

fn enter_namespace(pid: i32) -> NsOutcome {
    let user_fd = match fs::File::open(format!("/proc/{pid}/ns/user")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[STEAM NS] Failed to open user namespace: {e}");
            return NsOutcome::Unavailable;
        }
    };
    let pid_fd = match fs::File::open(format!("/proc/{pid}/ns/pid")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[STEAM NS] Failed to open PID namespace: {e}");
            return NsOutcome::Unavailable;
        }
    };

    unsafe {
        if setns(user_fd.as_raw_fd(), CLONE_NEWUSER) != 0 {
            eprintln!(
                "[STEAM NS] setns(user) failed: {}",
                std::io::Error::last_os_error()
            );
            return NsOutcome::Unavailable;
        }
        if setns(pid_fd.as_raw_fd(), CLONE_NEWPID) != 0 {
            eprintln!(
                "[STEAM NS] setns(pid) failed: {}",
                std::io::Error::last_os_error()
            );
            return NsOutcome::Unavailable;
        }

        let child = fork();
        if child < 0 {
            eprintln!(
                "[STEAM NS] fork failed: {}",
                std::io::Error::last_os_error()
            );
            return NsOutcome::Unavailable;
        }

        if child == 0 {
            NsOutcome::Entered
        } else {
            // The parent stub stays in the host PID namespace; its inherited copy
            // of the IPC pipe FDs closes when it exits, signaling EOF to the frontend.
            let mut status: c_int = 0;
            waitpid(child, &mut status, 0);
            let code = if status & 0x7f == 0 {
                ((status >> 8) & 0xff) as u8
            } else {
                1
            };
            NsOutcome::ParentExit(code)
        }
    }
}
