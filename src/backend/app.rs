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

use crate::backend::app_manager::AppManager;
use crate::backend::progress_io::{apply_app_export, collect_app_export};
use crate::dev_println;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::ipc_types::{SamError, SteamCommand, SteamResponse, read_message, write_message};
use interprocess::unnamed_pipe::{Recver, Sender};
use serde::Serialize;

fn send_response<T: Serialize>(tx: &mut Sender, resp: &SteamResponse<T>) {
    write_message(tx, resp).expect("[APP SERVER] Failed to send response");
}

/// Guard `app_id_param == expected`, then run `f` and send its `Result` as a
/// `SteamResponse`. Centralizes the four-step pattern (check id, call, wrap,
/// send) the per-command arms used to repeat.
fn dispatch<T: Serialize>(
    tx: &mut Sender,
    app_id_param: u32,
    expected: u32,
    f: impl FnOnce() -> Result<T, SamError>,
) {
    if app_id_param != expected {
        dev_println!("APPSRV", "App ID mismatch: {app_id_param} != {expected}");
        send_response::<()>(tx, &SteamResponse::Error(SamError::AppMismatchError));
        return;
    }
    let result = f();
    if let Err(e) = &result {
        dev_println!("APPSRV", "Command failed: {e}");
    }
    send_response(tx, &SteamResponse::<T>::from(result));
}

pub fn app(app_id: AppId_t, parent_tx: &mut Sender, parent_rx: &mut Recver) -> u8 {
    let mut app_manager = AppManager::new_connected(app_id);

    #[cfg(debug_assertions)]
    if app_manager.as_ref().is_err() {
        dev_println!("APPSRV", "Failed to connect to Steam");
    }

    loop {
        dev_println!("APPSRV", "Main loop...");

        let command: SteamCommand = match read_message(parent_rx) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[APP SERVER] Parent pipe error: {e} — shutting down");
                return 1;
            }
        };

        match &command {
            SteamCommand::Shutdown => {
                send_response(parent_tx, &SteamResponse::Success(true));
                break;
            }
            SteamCommand::Status => {
                let resp: SteamResponse<bool> = if app_manager.is_ok() {
                    SteamResponse::Success(true)
                } else {
                    SteamResponse::Error(SamError::SteamConnectionFailed)
                };
                send_response(parent_tx, &resp);
                continue;
            }
            _ => {}
        }

        if app_manager.as_ref().is_err() {
            send_response::<()>(
                parent_tx,
                &SteamResponse::Error(SamError::SteamConnectionFailed),
            );
            continue;
        }

        let am = app_manager.as_mut().unwrap();

        match command {
            SteamCommand::GetAchievements(id) => {
                dispatch(parent_tx, id, app_id, || am.get_achievements(true))
            }
            SteamCommand::GetStats(id) => dispatch(parent_tx, id, app_id, || am.get_statistics()),
            SteamCommand::SetAchievement(id, unlocked, ach_id, store) => {
                dispatch(parent_tx, id, app_id, || {
                    am.set_achievement(&ach_id, unlocked, store)
                })
            }
            SteamCommand::SetIntStat(id, stat_id, value) => {
                dispatch(parent_tx, id, app_id, || am.set_stat_i32(&stat_id, value))
            }
            SteamCommand::SetFloatStat(id, stat_id, value) => {
                dispatch(parent_tx, id, app_id, || am.set_stat_f32(&stat_id, value))
            }
            SteamCommand::StoreStatsAndAchievements(id) => dispatch(parent_tx, id, app_id, || {
                am.store_stats_and_achievements().map(|_| true)
            }),
            SteamCommand::ResetStats(id, achievements_too) => {
                dispatch(parent_tx, id, app_id, || {
                    am.reset_all_stats(achievements_too)
                })
            }
            SteamCommand::UnlockAllAchievements(id) => dispatch(parent_tx, id, app_id, || {
                am.unlock_all_achievements().map(|_| true)
            }),
            SteamCommand::ExportAppProgress(id) => {
                dispatch(parent_tx, id, app_id, || collect_app_export(am, app_id))
            }
            SteamCommand::ImportAppProgress(id, payload) => dispatch(parent_tx, id, app_id, || {
                Ok::<_, SamError>(apply_app_export(am, payload))
            }),
            SteamCommand::GetFriendUnlockTimes(id, friend) => {
                dispatch(parent_tx, id, app_id, || {
                    am.fetch_friend_unlock_times(&friend)
                })
            }
            SteamCommand::GetFriendAchievementCount(id, steam_id64) => {
                dispatch(parent_tx, id, app_id, || {
                    am.fetch_user_achievement_count(steam_id64)
                })
            }

            _ => {
                dev_println!("APPSRV", "Received unknown command {command:?}");
                send_response::<()>(parent_tx, &SteamResponse::Error(SamError::UnknownError));
            }
        }
    }

    dev_println!("APPSRV", "Exiting");

    0
}
