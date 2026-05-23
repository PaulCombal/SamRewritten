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

use crate::backend::app_lister::{AppLister, fetch_achievement_counts};
use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::local_config::parse_localconfig;
use crate::backend::local_stats::LocalIndex;
use crate::backend::progress_io::{MAX_CONCURRENT_APPS, run_command_on_apps_concurrent};
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::utils::app_paths::get_executable_path;
use crate::utils::bidir_child::BidirChild;
use crate::utils::ipc_client::IpcClient;
use crate::utils::ipc_types::{
    AppExport, ImportSummary, ProgressMsg, SamError, SteamCommand, SteamResponse, frame_message,
    parse_response_bytes, read_message, write_message,
};
use crate::utils::steam_locator::SteamLocator;
use interprocess::unnamed_pipe::{Recver, Sender};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::sync::{LazyLock, Mutex};

/// Forward `command` to the app server and return the framed response bytes
/// (length prefix + JSON) suitable for proxying straight back to the parent.
fn send_app_command(ipc: &mut IpcClient, command: SteamCommand) -> Result<Vec<u8>, SamError> {
    ipc.send(&command)?;
    ipc.recv_frame()
}

fn send<T: Serialize>(tx: &mut Sender, msg: &T) {
    write_message(tx, msg).expect("[ORCHESTRATOR] Failed to send response");
}

fn send_raw(tx: &mut Sender, bytes: &[u8]) {
    tx.write_all(bytes)
        .expect("[ORCHESTRATOR] Failed to send response");
}

pub fn orchestrator(parent_tx: &mut Sender, parent_rx: &mut Recver) -> u8 {
    // Lazy: only the app-list and achievement-count commands use the
    // orchestrator's own connection. Per-app commands go to child processes, so
    // a one-shot CLI call that forwards to a child pays for one handshake, not two.
    let mut connected_steam: Option<ConnectedSteam> = None;
    let mut children_processes: HashMap<u32, (IpcClient, usize)> = HashMap::new();

    loop {
        dev_println!("ORCH", "Main loop...");

        let message: SteamCommand = match read_message(parent_rx) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[ORCHESTRATOR] Parent pipe error: {e} — shutting down");
                for (_, (ipc, _)) in children_processes.iter_mut() {
                    let _ = send_app_command(ipc, SteamCommand::Shutdown);
                    let _ = ipc.wait();
                }
                return 1;
            }
        };

        dev_println!("ORCH", "Received message: {message:?}");

        let continue_running = process_command(
            message,
            parent_tx,
            &mut children_processes,
            &mut connected_steam,
        );
        if !continue_running {
            break 0;
        }
    }
}

fn ensure_connected(slot: &mut Option<ConnectedSteam>) -> Result<&mut ConnectedSteam, ()> {
    if slot.is_none() {
        // Refuse to connect unless Steam is running from our install
        // connecting to another live Steam crashes on the first app call.
        #[cfg(target_os = "linux")]
        if !crate::utils::steam_ns::loaded_install_is_running() {
            return Err(());
        }
        match ConnectedSteam::new(false) {
            Ok(c) => *slot = Some(c),
            Err(e) => {
                dev_println!("ORCH", "Error connecting to Steam: {e}");
                return Err(());
            }
        }
    }
    Ok(slot.as_mut().unwrap())
}

fn ensure_app_launched(
    app_id: u32,
    children_processes: &mut HashMap<u32, (IpcClient, usize)>,
) -> Result<(), SamError> {
    // If a process for this app is already alive, just bump the refcount.
    if let Some((_, refcount)) = children_processes.get_mut(&app_id) {
        *refcount += 1;
        dev_println!("ORCH", "App {} refcount now {}", app_id, *refcount);
        return Ok(());
    }

    // Otherwise launch a new process with refcount = 1.
    let current_exe = get_executable_path();
    let child =
        BidirChild::new(Command::new(current_exe).arg(format!("--app={app_id}"))).map_err(|e| {
            eprintln!("[ORCHESTRATOR] Failed to spawn app server for {app_id}: {e}");
            SamError::SocketCommunicationFailed
        })?;

    // Probe the child to verify it actually connected to Steam. The app
    // server's connect attempt happens before its main loop runs, so a Status
    // reply distinguishes a healthy child from one that failed to attach (e.g.
    // user-entered AppId they don't own).
    let mut ipc = IpcClient::new(child);
    match ipc.request_response::<bool, _>(&SteamCommand::Status) {
        Ok(true) => {
            children_processes.insert(app_id, (ipc, 1));
            Ok(())
        }
        Ok(false) | Err(_) => {
            dev_println!(
                "ORCH",
                "App server for {app_id} failed Steam handshake, tearing down"
            );
            let _ = send_app_command(&mut ipc, SteamCommand::Shutdown);
            let _ = ipc.wait();
            Err(SamError::SteamConnectionFailed)
        }
    }
}

/// Fetch a running child's achievements and stats in a single back-to-back
/// exchange, so nothing can interleave between the two on the parent channel.
fn fetch_child_progress(
    ipc: &mut IpcClient,
    app_id: u32,
) -> Result<(Vec<AchievementInfo>, Vec<StatInfo>), SamError> {
    let achievements =
        ipc.request_response::<Vec<AchievementInfo>, _>(&SteamCommand::GetAchievements(app_id))?;
    let stats = ipc.request_response::<Vec<StatInfo>, _>(&SteamCommand::GetStats(app_id))?;
    Ok((achievements, stats))
}

/// Forward `command` to the per-app child process for `app_id` and proxy its
/// framed response back to `tx` verbatim. If no child is running for `app_id`,
/// respond with `AppMismatchError`. If the IPC call to the child fails, respond
/// with `SocketCommunicationFailed`.
fn forward_to_child(
    app_id: u32,
    command: SteamCommand,
    tx: &mut Sender,
    children_processes: &mut HashMap<u32, (IpcClient, usize)>,
    op_name: &str,
) {
    if let Some((ipc, _)) = children_processes.get_mut(&app_id) {
        match send_app_command(ipc, command) {
            Ok(response) => send_raw(tx, &response),
            Err(_) => {
                dev_println!("ORCH", "Failed to {op_name} for app {app_id}");
                send_raw(tx, &SOCKET_ERROR_RESPONSE);
            }
        }
    } else {
        let _ = write_message(tx, &SteamResponse::<()>::Error(SamError::AppMismatchError));
    }
}

static SOCKET_ERROR_RESPONSE: LazyLock<Vec<u8>> = LazyLock::new(|| {
    frame_message(&SteamResponse::<()>::Error(
        SamError::SocketCommunicationFailed,
    ))
});

/// Children are spawned by us, so they inherit our namespace. Streams a
/// `ProgressMsg::Progress { done, total }` frame as each child completes, then
/// writes a single terminal `ProgressMsg::Done(SteamResponse::Success(results))`
/// — the GUI's `request_with_progress` reads frames in a loop until `Done`.
fn fan_out_streaming<T>(items: Vec<(u32, SteamCommand)>, tx: &mut Sender)
where
    T: DeserializeOwned + Serialize,
{
    let tx_lock = Mutex::new(tx);
    let progress = |done: usize, total: usize, _app_id: u32| {
        let mut guard = tx_lock.lock().unwrap();
        let _ = write_message(&mut **guard, &ProgressMsg::<()>::Progress { done, total });
    };
    let raw = run_command_on_apps_concurrent(items, MAX_CONCURRENT_APPS, Some(&progress));
    let results: Vec<(u32, Result<T, SamError>)> = raw
        .into_iter()
        .map(|(id, res)| (id, res.and_then(|bytes| parse_response_bytes::<T>(&bytes))))
        .collect();
    let tx = tx_lock.into_inner().unwrap();
    send(tx, &ProgressMsg::Done(SteamResponse::Success(results)));
}

fn process_command(
    command: SteamCommand,
    tx: &mut Sender,
    children_processes: &mut HashMap<u32, (IpcClient, usize)>,
    connected_steam: &mut Option<ConnectedSteam>,
) -> bool {
    /// One-shot: spawn an ephemeral app server, send `command`, then shut it
    /// down. Proxies the framed response (or a `SocketCommunicationFailed`
    /// envelope) back to `tx`. Used for commands the user can issue against
    /// apps that aren't currently being held open (unlock-all, reset-stats).
    fn run_ephemeral(tx: &mut Sender, app_id: u32, command: SteamCommand, op_name: &str) {
        let current_exe = get_executable_path();
        let child = match BidirChild::new(Command::new(current_exe).arg(format!("--app={app_id}")))
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ORCHESTRATOR] Failed to spawn app server for {op_name} {app_id}: {e}");
                send_raw(tx, &SOCKET_ERROR_RESPONSE);
                return;
            }
        };
        let mut ipc = IpcClient::new(child);

        let response = send_app_command(&mut ipc, command);

        if send_app_command(&mut ipc, SteamCommand::Shutdown).is_err() {
            dev_println!("ORCH", "Error sending shutdown to {op_name} app {app_id}");
            send_raw(tx, &SOCKET_ERROR_RESPONSE);
            return;
        }

        ipc.wait()
            .expect("[ORCHESTRATOR] Failed to wait child process");

        match response {
            Ok(resp) => send_raw(tx, &resp),
            Err(_) => send_raw(tx, &SOCKET_ERROR_RESPONSE),
        }
    }

    match command {
        SteamCommand::GetSubscribedAppList(include_playtime, with_achievement_counts) => {
            dev_println!(
                "ORCH",
                "Received GetSubscribedAppList(playtime={include_playtime}, achievements={with_achievement_counts})"
            );

            #[cfg(target_os = "linux")]
            if connected_steam.is_some() && !crate::utils::steam_ns::loaded_install_is_running() {
                *connected_steam = None;
            }

            let connected_steam = match ensure_connected(connected_steam) {
                Ok(cs) => cs,
                Err(()) => {
                    send(tx, &SteamResponse::<()>::Error(SamError::SteamConnectionFailed));
                    return true;
                }
            };

            let vdf_path = if include_playtime {
                match connected_steam.user.get_steam_id() {
                    Ok(steam_id) => {
                        let account_id = (steam_id.m_steamid & 0xFFFF_FFFF) as u32;
                        let path = SteamLocator::get_local_config_path(account_id);
                        if path.is_none() {
                            dev_println!(
                                "ORCH",
                                "localconfig.vdf not found for account {account_id}"
                            );
                        }
                        path
                    }
                    Err(e) => {
                        dev_println!("ORCH", "Failed to get Steam ID: {e:?}");
                        None
                    }
                }
            } else {
                None
            };

            let vdf_handle =
                vdf_path.map(|path| std::thread::spawn(move || parse_localconfig(&path)));

            let apps_001 = &connected_steam.apps_001;
            let apps = &connected_steam.apps;
            let app_lister = AppLister::new(apps_001, apps);

            let client_user = match connected_steam.client_user() {
                Ok(u) => u,
                Err(e) => {
                    dev_println!("ORCH", "Could not get IClientUser: {e}");
                    send(tx, &SteamResponse::<()>::Error(SamError::SteamConnectionFailed));
                    return true;
                }
            };

            let stats_map = if with_achievement_counts {
                match connected_steam.client_user_stats_map() {
                    Ok(m) => Some(m),
                    Err(e) => {
                        dev_println!("ORCH", "Could not create stats map: {e}");
                        None
                    }
                }
            } else {
                None
            };
            let result = app_lister.get_owned_apps(&client_user, stats_map.as_ref());

            match result {
                Ok(mut apps) => {
                    if let Some(handle) = vdf_handle {
                        match handle.join() {
                            Ok(Ok(map)) => {
                                for app in &mut apps {
                                    if let Some(entry) = map.get(&app.app_id) {
                                        app.playtime_minutes = entry.playtime_minutes;
                                        app.last_played = entry.last_played;
                                    }
                                }
                            }
                            Ok(Err(_)) => {
                                // parse_localconfig already logged via dev_println!
                            }
                            Err(_) => {
                                eprintln!("[ORCHESTRATOR] VDF parse thread panicked");
                            }
                        }
                    }
                    send(tx, &SteamResponse::Success(apps));
                }
                Err(e) => {
                    dev_println!("ORCH", "Error getting owned apps: {e}");
                    send(tx, &SteamResponse::<()>::Error(e));
                }
            };
        }

        SteamCommand::LaunchApp(app_id) => {
            dev_println!("ORCH", "LaunchApp {}", app_id);

            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(tx, &SteamResponse::<bool>::Success(true));
                return true;
            }

            match ensure_app_launched(app_id, children_processes) {
                Ok(()) => send(tx, &SteamResponse::Success(true)),
                Err(e) => send(tx, &SteamResponse::<bool>::Error(e)),
            }
        }

        SteamCommand::GetAchievementsAndStats(app_id, launch) => {
            dev_println!(
                "ORCH",
                "GetAchievementsAndStats {} (launch={launch})",
                app_id
            );

            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(
                    tx,
                    &SteamResponse::Success((
                        Vec::<AchievementInfo>::new(),
                        Vec::<StatInfo>::new(),
                    )),
                );
                return true;
            }

            if launch && let Err(e) = ensure_app_launched(app_id, children_processes) {
                send(
                    tx,
                    &SteamResponse::<(Vec<AchievementInfo>, Vec<StatInfo>)>::Error(e),
                );
                return true;
            }

            // One orchestrator command holds the channel for both fetches, so no
            // other command can wedge in between them.
            let Some((ipc, _)) = children_processes.get_mut(&app_id) else {
                send(
                    tx,
                    &SteamResponse::<(Vec<AchievementInfo>, Vec<StatInfo>)>::Error(
                        SamError::AppMismatchError,
                    ),
                );
                return true;
            };

            match fetch_child_progress(ipc, app_id) {
                Ok(progress) => send(tx, &SteamResponse::Success(progress)),
                Err(_) => send_raw(tx, &SOCKET_ERROR_RESPONSE),
            }
        }

        SteamCommand::StopApp(app_id) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(tx, &SteamResponse::<bool>::Success(true));
                return true;
            }

            let Some((_, refcount)) = children_processes.get_mut(&app_id) else {
                eprintln!("[ORCHESTRATOR] App {} is not running", app_id);
                send(tx, &SteamResponse::<()>::Error(SamError::UnknownError));
                return true;
            };

            *refcount -= 1;
            if *refcount > 0 {
                dev_println!(
                    "ORCH",
                    "App {} still wanted, refcount now {}",
                    app_id,
                    *refcount
                );
                send(tx, &SteamResponse::Success(true));
                return true;
            }

            // Refcount hit zero — actually shut the process down.
            let mut ipc_opt = children_processes.remove(&app_id).map(|(b, _)| b);
            let ipc = ipc_opt.as_mut().unwrap();
            let response = match send_app_command(ipc, SteamCommand::Shutdown) {
                Ok(response) => response,
                Err(_) => {
                    dev_println!("ORCH", "Error sending shutdown command to app {app_id}");
                    send_raw(tx, &SOCKET_ERROR_RESPONSE);
                    return true;
                }
            };

            ipc.wait()
                .expect("[ORCHESTRATOR] Failed to wait child process");

            send_raw(tx, &response);
        }

        SteamCommand::StopApps => {
            dev_println!("ORCH", "StopApps");

            for (app_id, (ipc, _)) in children_processes.iter_mut() {
                let _ = send_app_command(ipc, SteamCommand::Shutdown);
                dev_println!("ORCH", "Sent shutdown command to app {app_id}");
                ipc.wait()
                    .expect("[ORCHESTRATOR] Failed to wait child process");
            }

            children_processes.clear();

            send(tx, &SteamResponse::Success(true));
        }

        SteamCommand::Shutdown => {
            for (app_id, (ipc, _)) in children_processes.iter_mut() {
                let _ = send_app_command(ipc, SteamCommand::Shutdown);
                dev_println!("ORCH", "Sent shutdown command to app {app_id}");
                ipc.wait()
                    .expect("[ORCHESTRATOR] Failed to wait child process");
            }

            send(tx, &SteamResponse::Success(true));
            return false;
        }

        SteamCommand::Status => {
            send(tx, &SteamResponse::Success(true));
        }

        SteamCommand::GetRunningApps => {
            let running: Vec<u32> = children_processes.keys().copied().collect();
            send(tx, &SteamResponse::Success(running));
        }

        SteamCommand::GetAchievements(app_id) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                let mut ach_infos = vec![];
                for i in 1..1000 {
                    let ach_info = AchievementInfo {
                        id: format!("DEV_ACH_{i}"),
                        is_achieved: (i % 2) == 0,
                        name: format!("Development achievement {i}"),
                        global_achieved_percent: None,
                        permission: 0,
                        description: "Description".to_string(),
                        icon_locked: "".to_string(),
                        icon_normal: "".to_string(),
                        unlock_time: None,
                    };
                    ach_infos.push(ach_info);
                }

                send(
                    tx,
                    &SteamResponse::<Vec<AchievementInfo>>::Success(ach_infos),
                );
                return true;
            }

            forward_to_child(
                app_id,
                SteamCommand::GetAchievements(app_id),
                tx,
                children_processes,
                "load achievements",
            );
        }

        SteamCommand::GetStats(app_id) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(tx, &SteamResponse::<Vec<StatInfo>>::Success(vec![]));
                return true;
            }

            forward_to_child(
                app_id,
                SteamCommand::GetStats(app_id),
                tx,
                children_processes,
                "load stats",
            );
        }

        SteamCommand::SetAchievement(app_id, unlocked, achievement_id, store) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(tx, &SteamResponse::<bool>::Success(true));
                return true;
            }

            forward_to_child(
                app_id,
                SteamCommand::SetAchievement(app_id, unlocked, achievement_id, store),
                tx,
                children_processes,
                "set achievement",
            );
        }

        SteamCommand::UnlockAllAchievements(app_id) => {
            run_ephemeral(
                tx,
                app_id,
                SteamCommand::UnlockAllAchievements(app_id),
                "unlock-all",
            );
        }

        SteamCommand::StoreStatsAndAchievements(app_id) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                send(tx, &SteamResponse::<bool>::Success(true));
                return true;
            }

            forward_to_child(
                app_id,
                SteamCommand::StoreStatsAndAchievements(app_id),
                tx,
                children_processes,
                "store stats",
            );
        }

        SteamCommand::SetIntStat(app_id, stat_id, value) => {
            forward_to_child(
                app_id,
                SteamCommand::SetIntStat(app_id, stat_id, value),
                tx,
                children_processes,
                "set int stat",
            );
        }

        SteamCommand::SetFloatStat(app_id, stat_id, value) => {
            forward_to_child(
                app_id,
                SteamCommand::SetFloatStat(app_id, stat_id, value),
                tx,
                children_processes,
                "set float stat",
            );
        }

        SteamCommand::ResetStats(app_id, achievements_too) => {
            if children_processes.contains_key(&app_id) {
                forward_to_child(
                    app_id,
                    SteamCommand::ResetStats(app_id, achievements_too),
                    tx,
                    children_processes,
                    "reset stats",
                );
            } else {
                run_ephemeral(
                    tx,
                    app_id,
                    SteamCommand::ResetStats(app_id, achievements_too),
                    "reset-stats",
                );
            }
        }

        SteamCommand::GetAchievementCounts(app_ids) => {
            let connected_steam = match ensure_connected(connected_steam) {
                Ok(cs) => cs,
                Err(()) => {
                    send(tx, &SteamResponse::<()>::Error(SamError::SteamConnectionFailed));
                    return true;
                }
            };

            // Local-disk fast path; IPC fallback for misses.
            let local_index = connected_steam
                .user
                .get_steam_id()
                .ok()
                .map(|sid| (sid.m_steamid & 0xFFFF_FFFF) as u32)
                .and_then(LocalIndex::build);

            let mut counts: Vec<(u32, u32, u32)> = Vec::with_capacity(app_ids.len());
            let mut remaining: Vec<u32> = Vec::new();
            if let Some(index) = &local_index {
                for &app_id in &app_ids {
                    match index.try_read(app_id) {
                        Some((total, unlocked)) => counts.push((app_id, total, unlocked)),
                        None => remaining.push(app_id),
                    }
                }
            } else {
                remaining = app_ids.clone();
            }

            if !remaining.is_empty() {
                let stats_map = match connected_steam.client_user_stats_map() {
                    Ok(m) => m,
                    Err(e) => {
                        dev_println!("ORCH", "Could not create stats map: {e}");
                        send(tx, &SteamResponse::<()>::Error(SamError::SteamConnectionFailed));
                        return true;
                    }
                };
                counts.extend(fetch_achievement_counts(&stats_map, &remaining));
            }

            send(tx, &SteamResponse::Success(counts));
        }

        SteamCommand::ExportApps(app_ids) => {
            dev_println!("ORCH", "ExportApps {:?}", app_ids);
            let items = app_ids
                .into_iter()
                .map(|id| (id, SteamCommand::ExportAppProgress(id)))
                .collect();
            fan_out_streaming::<AppExport>(items, tx);
        }

        SteamCommand::ImportApps(apps) => {
            dev_println!("ORCH", "ImportApps ({} apps)", apps.len());
            let items = apps
                .into_iter()
                .map(|a| (a.app_id, SteamCommand::ImportAppProgress(a.app_id, a)))
                .collect();
            fan_out_streaming::<ImportSummary>(items, tx);
        }

        SteamCommand::UnlockAllApps(app_ids) => {
            dev_println!("ORCH", "UnlockAllApps {:?}", app_ids);
            let items = app_ids
                .into_iter()
                .map(|id| (id, SteamCommand::UnlockAllAchievements(id)))
                .collect();
            fan_out_streaming::<bool>(items, tx);
        }

        SteamCommand::ResetApps(app_ids, achievements_too) => {
            dev_println!("ORCH", "ResetApps {:?}", app_ids);
            let items = app_ids
                .into_iter()
                .map(|id| (id, SteamCommand::ResetStats(id, achievements_too)))
                .collect();
            fan_out_streaming::<bool>(items, tx);
        }

        // Child-only commands. The orchestrator dispatches these to app
        // server children via `run_command_on_apps_concurrent`; receiving
        // one here means a caller mistakenly addressed the orchestrator.
        SteamCommand::ExportAppProgress(_) | SteamCommand::ImportAppProgress(_, _) => {
            dev_println!("ORCH", "Received child-only command");
            send_raw(tx, &SOCKET_ERROR_RESPONSE);
        }
    };

    true
}
