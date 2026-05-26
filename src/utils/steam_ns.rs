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
//! own uid created, so we can join it without root. Two paths exist:
//!
//! * **Preferred (Linux 5.8+):** open a pidfd for the Flatpak Steam process and
//!   ask the kernel to install both namespaces atomically with
//!   `setns(pidfd, CLONE_NEWUSER | CLONE_NEWPID)`. The kernel installs the new
//!   credentials and then the PID namespace within a single transaction. Newer
//!   kernels (notably Fedora's) require this — they refuse the separate
//!   `setns(CLONE_NEWPID)` step below with EPERM even when we hold
//!   CAP_SYS_ADMIN in the joined user namespace. This is the path modern
//!   `nsenter` uses.
//! * **Fallback:** the historical two-step — `setns` into the user namespace
//!   (granting CAP_SYS_ADMIN there), then the PID namespace. Kept for kernels
//!   that don't accept the multi-flag form.
//!
//! Either way the PID-namespace switch only takes effect for children, so we
//! `fork` after joining and the child becomes the orchestrator. We keep the
//! host mount namespace, so our binary and the Flatpak `steamclient.so` stay
//! reachable, and the network namespace is already shared (Steam's IPC is
//! loopback TCP).
//!
//! Only the orchestrator calls this, at startup before any threads exist; the
//! app-server children it spawns inherit the namespace.

use crate::dev_println;
use crate::utils::steam_locator::SteamLocator;
use std::fs;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::raw::{c_int, c_long};
use std::path::{Path, PathBuf};

const CLONE_NEWUSER: c_int = 0x1000_0000;
const CLONE_NEWPID: c_int = 0x2000_0000;

// pidfd_open(2). Same syscall number across Linux's modern uniform table
// (x86_64, aarch64, riscv64, arm, i386, ppc64, s390x, …); Linux 5.3+.
const SYS_PIDFD_OPEN: c_long = 434;

unsafe extern "C" {
    fn setns(fd: c_int, nstype: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn syscall(num: c_long, ...) -> c_long;
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
    // Resolve via the install root, not the loaded steamclient.so path: the snap
    // dlopens a copy from $SNAP_USER_COMMON, so the .so path isn't the real
    // install. Skip dirs without a steamclient.so to match prior behaviour.
    let Some(target) = SteamLocator::get_local_steam_install_root_folders()
        .into_iter()
        .find(|root| root.join("linux64/steamclient.so").exists())
        .and_then(|root| fs::canonicalize(root).ok())
    else {
        return false;
    };

    let roots = running_steam_install_roots();
    if !roots.is_empty() {
        return roots.contains(&target);
    }

    // No install identifiable — typically the snap, where AppArmor allows
    // /proc/<pid>/comm but denies /proc/<pid>/maps for other snaps' processes.
    // Fall back to "a steam process exists"; plug scoping keeps us to one install.
    any_steam_process_running()
}

fn any_steam_process_running() -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
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
            == "steam"
        {
            return true;
        }
    }
    false
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
    if try_atomic_join(pid) {
        return fork_after_join();
    }
    try_two_step_join(pid)
}

/// Preferred path: install user+pid namespaces atomically via a pidfd. Returns
/// `true` on success; logs the reason and returns `false` on any failure so the
/// caller can try the legacy two-step path. The pidfd is closed on return
/// (success or failure) via `OwnedFd`'s `Drop`.
fn try_atomic_join(pid: i32) -> bool {
    let raw = unsafe { syscall(SYS_PIDFD_OPEN, pid as c_long, 0 as c_long) };
    if raw < 0 {
        dev_println!(
            "STEAM NS",
            "pidfd_open({pid}) unavailable ({}); trying two-step setns",
            std::io::Error::last_os_error()
        );
        return false;
    }
    // SAFETY: `raw` is a valid fd we just received from a successful
    // `pidfd_open` syscall; nothing else owns it.
    let pidfd = unsafe { OwnedFd::from_raw_fd(raw as c_int) };
    let rc = unsafe { setns(pidfd.as_raw_fd(), CLONE_NEWUSER | CLONE_NEWPID) };
    if rc != 0 {
        dev_println!(
            "STEAM NS",
            "atomic setns(NEWUSER|NEWPID) failed ({}); trying two-step setns",
            std::io::Error::last_os_error()
        );
        return false;
    }
    true
}

/// Legacy path for kernels that don't accept multi-flag `setns`: enter the
/// user namespace first (gaining CAP_SYS_ADMIN there), then the PID namespace.
/// On newer kernels that reject the second call (e.g. Fedora 44), this leaves
/// us in the new user namespace with the host PID namespace — same partial
/// state as before this dispatcher existed; the caller surfaces `Unavailable`
/// and the IPC pipe will then fail with the old broken-pipe symptom.
fn try_two_step_join(pid: i32) -> NsOutcome {
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
    }
    fork_after_join()
}

/// `setns(CLONE_NEWPID)` only takes effect for new children, so we fork once
/// the namespaces are installed; the child becomes the orchestrator. The
/// parent stub stays in the host PID namespace and exits with the child's
/// status — its inherited copy of the IPC pipe FDs closes on exit, signaling
/// EOF to the frontend.
fn fork_after_join() -> NsOutcome {
    unsafe {
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
