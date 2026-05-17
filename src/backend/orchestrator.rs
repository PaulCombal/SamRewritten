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
#[cfg(debug_assertions)]
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::utils::app_paths::get_executable_path;
use crate::utils::bidir_child::BidirChild;
use crate::utils::ipc_client::IpcClient;
use crate::utils::ipc_types::{
    SamError, SteamCommand, SteamResponse, frame_message, read_message, write_message,
};
use crate::utils::steam_locator::SteamLocator;
use interprocess::unnamed_pipe::{Recver, Sender};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::sync::LazyLock;

/// Forward `command` to the app server and return the framed response bytes
/// (length prefix + JSON) suitable for proxying straight back to the parent.
fn send_app_command(ipc: &mut IpcClient, command: SteamCommand) -> Result<Vec<u8>, SamError> {
    ipc.send(&command)?;
    ipc.recv_frame()
}

pub fn orchestrator(parent_tx: &mut Sender, parent_rx: &mut Recver) -> u8 {
    let mut connected_steam: Option<ConnectedSteam> = None;
    let mut children_processes: HashMap<u32, (IpcClient, usize)> = HashMap::new();

    loop {
        dev_println!("[ORCHESTRATOR] Main loop...");

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

        dev_println!("[ORCHESTRATOR] Received message: {message:?}");

        if connected_steam.as_ref().is_none() {
            if message == SteamCommand::Shutdown {
                let _ = write_message(parent_tx, &SteamResponse::Success(true));
                dev_println!("[ORCHESTRATOR] Exiting");
                break 0;
            }

            connected_steam = match ConnectedSteam::new(false) {
                Ok(c) => Some(c),
                Err(e) => {
                    dev_println!("[ORCHESTRATOR] Error connecting to Steam: {e}");
                    let _ = write_message(
                        parent_tx,
                        &SteamResponse::<String>::Error(SamError::SteamConnectionFailed),
                    );
                    continue;
                }
            };
        }

        let cs = connected_steam.as_mut();
        let cs = cs.unwrap();
        let continue_running = process_command(message, parent_tx, &mut children_processes, cs);
        if !continue_running {
            break 0;
        }
    }
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
            Ok(response) => {
                tx.write_all(&response)
                    .expect("[ORCHESTRATOR] Failed to send response");
            }
            Err(_) => {
                dev_println!("[ORCHESTRATOR] Failed to {op_name} for app {app_id}");
                tx.write_all(&SOCKET_ERROR_RESPONSE)
                    .expect("[ORCHESTRATOR] Failed to send response");
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

fn process_command(
    command: SteamCommand,
    tx: &mut Sender,
    children_processes: &mut HashMap<u32, (IpcClient, usize)>,
    connected_steam: &mut ConnectedSteam,
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
                tx.write_all(&SOCKET_ERROR_RESPONSE)
                    .expect("[ORCHESTRATOR] Failed to send response");
                return;
            }
        };
        let mut ipc = IpcClient::new(child);

        // Just to be sure (this is probably useless)
        std::thread::sleep(std::time::Duration::from_millis(10));
        let response = send_app_command(&mut ipc, command);
        // Just to be sure (this is probably useless)
        std::thread::sleep(std::time::Duration::from_millis(10));

        if send_app_command(&mut ipc, SteamCommand::Shutdown).is_err() {
            dev_println!("[ORCHESTRATOR] Error sending shutdown to {op_name} app {app_id}");
            tx.write_all(&SOCKET_ERROR_RESPONSE)
                .expect("[ORCHESTRATOR] Failed to send response");
            return;
        }

        ipc.wait()
            .expect("[ORCHESTRATOR] Failed to wait child process");

        match response {
            Ok(resp) => tx
                .write_all(&resp)
                .expect("[ORCHESTRATOR] Failed to send response"),
            Err(_) => tx
                .write_all(&SOCKET_ERROR_RESPONSE)
                .expect("[ORCHESTRATOR] Failed to send response"),
        }
    }

    match command {
        SteamCommand::GetSubscribedAppList(include_playtime, with_achievement_counts) => {
            dev_println!(
                "[ORCHESTRATOR] Received GetSubscribedAppList(playtime={include_playtime}, achievements={with_achievement_counts})"
            );

            let vdf_path = if include_playtime {
                match connected_steam.user.get_steam_id() {
                    Ok(steam_id) => {
                        let account_id = (steam_id.m_steamid & 0xFFFF_FFFF) as u32;
                        let path = SteamLocator::get_local_config_path(account_id);
                        if path.is_none() {
                            dev_println!(
                                "[ORCHESTRATOR] localconfig.vdf not found for account {account_id}"
                            );
                        }
                        path
                    }
                    Err(e) => {
                        dev_println!("[ORCHESTRATOR] Failed to get Steam ID: {e:?}");
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
                    dev_println!("[ORCHESTRATOR] Could not get IClientUser: {e}");
                    write_message(
                        tx,
                        &SteamResponse::<()>::Error(SamError::SteamConnectionFailed),
                    )
                    .expect("[ORCHESTRATOR] Failed to send response");
                    return true;
                }
            };

            let stats_map = if with_achievement_counts {
                match connected_steam.client_user_stats_map() {
                    Ok(m) => Some(m),
                    Err(e) => {
                        dev_println!("[ORCHESTRATOR] Could not create stats map: {e}");
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
                    write_message(tx, &SteamResponse::Success(apps))
                        .expect("[ORCHESTRATOR] Failed to send response");
                }
                Err(e) => {
                    dev_println!("[ORCHESTRATOR] Error getting owned apps: {e}");
                    write_message(tx, &SteamResponse::<()>::Error(e))
                        .expect("[ORCHESTRATOR] Failed to send response");
                }
            };
        }

        SteamCommand::LaunchApp(app_id) => {
            dev_println!("[ORCHESTRATOR] LaunchApp {}", app_id);

            #[cfg(debug_assertions)]
            if app_id == 0 {
                write_message(tx, &SteamResponse::<bool>::Success(true))
                    .expect("[APP SERVER] Failed to send response");
                return true;
            }

            // 1. If a process for this app is already alive, just bump the refcount.
            if let Some((_, refcount)) = children_processes.get_mut(&app_id) {
                *refcount += 1;
                dev_println!("[ORCHESTRATOR] App {} refcount now {}", app_id, *refcount);
                write_message(tx, &SteamResponse::Success(true))
                    .expect("[ORCHESTRATOR] Failed to send response");
                return true;
            }

            // 2. Otherwise launch a new process with refcount = 1.
            let current_exe = get_executable_path();
            let child =
                match BidirChild::new(Command::new(current_exe).arg(format!("--app={app_id}"))) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[ORCHESTRATOR] Failed to spawn app server for {app_id}: {e}");
                        tx.write_all(&SOCKET_ERROR_RESPONSE)
                            .expect("[ORCHESTRATOR] Failed to send response");
                        return true;
                    }
                };

            // Probe the child to verify it actually connected to Steam. The
            // app server's connect attempt happens before its main loop runs,
            // so a Status reply distinguishes a healthy child from one that
            // failed to attach (e.g. user-entered AppId they don't own).
            let mut ipc = IpcClient::new(child);
            match ipc.request_response::<bool, _>(&SteamCommand::Status) {
                Ok(true) => {
                    children_processes.insert(app_id, (ipc, 1));
                    write_message(tx, &SteamResponse::Success(true))
                        .expect("[ORCHESTRATOR] Failed to send response");
                }
                Ok(false) | Err(_) => {
                    dev_println!(
                        "[ORCHESTRATOR] App server for {app_id} failed Steam handshake, tearing down"
                    );
                    let _ = send_app_command(&mut ipc, SteamCommand::Shutdown);
                    let _ = ipc.wait();
                    write_message(
                        tx,
                        &SteamResponse::<()>::Error(SamError::SteamConnectionFailed),
                    )
                    .expect("[ORCHESTRATOR] Failed to send response");
                }
            }
        }

        SteamCommand::StopApp(app_id) => {
            #[cfg(debug_assertions)]
            if app_id == 0 {
                write_message(tx, &SteamResponse::<bool>::Success(true))
                    .expect("[APP SERVER] Failed to send response");
                return true;
            }

            let Some((_, refcount)) = children_processes.get_mut(&app_id) else {
                eprintln!("[ORCHESTRATOR] App {} is not running", app_id);
                write_message(tx, &SteamResponse::<()>::Error(SamError::UnknownError))
                    .expect("[ORCHESTRATOR] Failed to send response");
                return true;
            };

            *refcount -= 1;
            if *refcount > 0 {
                dev_println!(
                    "[ORCHESTRATOR] App {} still wanted, refcount now {}",
                    app_id,
                    *refcount
                );
                write_message(tx, &SteamResponse::Success(true))
                    .expect("[ORCHESTRATOR] Failed to send response");
                return true;
            }

            // Refcount hit zero — actually shut the process down.
            let mut ipc_opt = children_processes.remove(&app_id).map(|(b, _)| b);
            let ipc = ipc_opt.as_mut().unwrap();
            let response = match send_app_command(ipc, SteamCommand::Shutdown) {
                Ok(response) => response,
                Err(_) => {
                    dev_println!("[ORCHESTRATOR] Error sending shutdown command to app {app_id}");
                    tx.write_all(&SOCKET_ERROR_RESPONSE)
                        .expect("[ORCHESTRATOR] Failed to send response");
                    return true;
                }
            };

            ipc.wait()
                .expect("[ORCHESTRATOR] Failed to wait child process");

            tx.write_all(&response)
                .expect("[ORCHESTRATOR] Failed to send response");
        }

        SteamCommand::StopApps => {
            dev_println!("[ORCHESTRATOR] StopApps");

            for (app_id, (ipc, _)) in children_processes.iter_mut() {
                let _ = send_app_command(ipc, SteamCommand::Shutdown);
                dev_println!("[ORCHESTRATOR] Sent shutdown command to app {app_id}");
                ipc.wait()
                    .expect("[ORCHESTRATOR] Failed to wait child process");
            }

            children_processes.clear();

            write_message(tx, &SteamResponse::Success(true))
                .expect("[ORCHESTRATOR] Failed to send response");
        }

        SteamCommand::Shutdown => {
            for (app_id, (ipc, _)) in children_processes.iter_mut() {
                let _ = send_app_command(ipc, SteamCommand::Shutdown);
                dev_println!("[ORCHESTRATOR] Sent shutdown command to app {app_id}");
                ipc.wait()
                    .expect("[ORCHESTRATOR] Failed to wait child process");
            }

            write_message(tx, &SteamResponse::Success(true))
                .expect("[ORCHESTRATOR] Failed to send response");
            return false;
        }

        SteamCommand::Status => {
            write_message(tx, &SteamResponse::Success(true))
                .expect("[ORCHESTRATOR] Failed to send response");
        }

        SteamCommand::GetRunningApps => {
            let running: Vec<u32> = children_processes.keys().copied().collect();
            write_message(tx, &SteamResponse::Success(running))
                .expect("[ORCHESTRATOR] Failed to send response");
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

                write_message(
                    tx,
                    &SteamResponse::<Vec<AchievementInfo>>::Success(ach_infos),
                )
                .expect("[APP SERVER] Failed to send response");
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
                write_message(tx, &SteamResponse::<Vec<StatInfo>>::Success(vec![]))
                    .expect("[APP SERVER] Failed to send response");
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
                write_message(tx, &SteamResponse::<bool>::Success(true))
                    .expect("[APP SERVER] Failed to send response");
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
                write_message(tx, &SteamResponse::<bool>::Success(true))
                    .expect("[APP SERVER] Failed to send response");
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
            dev_println!(
                "[ORCHESTRATOR] GetAchievementCounts ({} ids)",
                app_ids.len()
            );

            let stats_map = match connected_steam.client_user_stats_map() {
                Ok(m) => m,
                Err(e) => {
                    dev_println!("[ORCHESTRATOR] Could not create stats map: {e}");
                    write_message(
                        tx,
                        &SteamResponse::<()>::Error(SamError::SteamConnectionFailed),
                    )
                    .expect("[ORCHESTRATOR] Failed to send response");
                    return true;
                }
            };

            let counts = fetch_achievement_counts(&stats_map, &app_ids);
            write_message(tx, &SteamResponse::Success(counts))
                .expect("[ORCHESTRATOR] Failed to send response");
        }

        // Child-only commands. The orchestrator dispatches these to app
        // server children via `run_command_on_apps_concurrent`; receiving
        // one here means a caller mistakenly addressed the orchestrator.
        SteamCommand::ExportAppProgress(_) | SteamCommand::ImportAppProgress(_, _) => {
            dev_println!("[ORCHESTRATOR] Received child-only command");
            tx.write_all(&SOCKET_ERROR_RESPONSE)
                .expect("[ORCHESTRATOR] Failed to send response");
        }
    };

    true
}
