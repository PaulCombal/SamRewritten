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
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::ipc_types::{
    AppExport, SamError, SteamCommand, SteamResponse, read_message, write_message,
};
use interprocess::unnamed_pipe::{Recver, Sender};
use serde::Serialize;

fn send_response<T: Serialize>(tx: &mut Sender, resp: &SteamResponse<T>) {
    write_message(tx, resp).expect("[APP SERVER] Failed to send response");
}

fn check_app_id(passed: u32, expected: u32, tx: &mut Sender) -> bool {
    if passed != expected {
        dev_println!("[APP SERVER] App ID mismatch: {passed} != {expected}");
        send_response::<()>(tx, &SteamResponse::Error(SamError::AppMismatchError));
        return false;
    }
    true
}

pub fn app(app_id: AppId_t, parent_tx: &mut Sender, parent_rx: &mut Recver) -> u8 {
    let mut app_manager = AppManager::new_connected(app_id);

    #[cfg(debug_assertions)]
    if app_manager.as_ref().is_err() {
        dev_println!("[APP SERVER] Failed to connect to Steam");
    }

    loop {
        dev_println!("[APP SERVER] Main loop...");

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

        let app_manager = app_manager.as_mut().unwrap();

        match command {
            SteamCommand::GetAchievements(app_id_param) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<Vec<AchievementInfo>> =
                    match app_manager.get_achievements(true) {
                        Ok(achievements) => SteamResponse::Success(achievements),
                        Err(e) => SteamResponse::Error(e),
                    };
                send_response(parent_tx, &response);
            }

            SteamCommand::GetStats(app_id_param) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<Vec<StatInfo>> = match app_manager.get_statistics() {
                    Ok(statistics) => SteamResponse::Success(statistics),
                    Err(e) => SteamResponse::Error(e),
                };
                send_response(parent_tx, &response);
            }

            SteamCommand::SetAchievement(app_id_param, unlocked, achievement_id, store) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> =
                    match app_manager.set_achievement(&achievement_id, unlocked, store) {
                        Ok(_) => SteamResponse::Success(true),
                        Err(e) => {
                            dev_println!("[APP SERVER] Error setting achievement: {e}");
                            SteamResponse::Error(e)
                        }
                    };
                send_response(parent_tx, &response);
            }

            SteamCommand::SetIntStat(app_id_param, stat_id, value) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> = match app_manager.set_stat_i32(&stat_id, value)
                {
                    Ok(result) => SteamResponse::Success(result),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting int stat: {e}");
                        SteamResponse::Error(e)
                    }
                };
                send_response(parent_tx, &response);
            }

            SteamCommand::SetFloatStat(app_id_param, stat_id, value) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> = match app_manager.set_stat_f32(&stat_id, value)
                {
                    Ok(result) => SteamResponse::Success(result),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting float stat: {e}");
                        SteamResponse::Error(e)
                    }
                };
                send_response(parent_tx, &response);
            }

            SteamCommand::StoreStatsAndAchievements(app_id_param) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> = match app_manager.store_stats_and_achievements()
                {
                    Ok(_) => SteamResponse::Success(true),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error storing stats and achievements: {e}");
                        SteamResponse::Error(e)
                    }
                };
                send_response(parent_tx, &response);
            }

            SteamCommand::ResetStats(app_id_param, achievements_too) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> =
                    match app_manager.reset_all_stats(achievements_too) {
                        Ok(result) => SteamResponse::Success(result),
                        Err(e) => {
                            dev_println!("[APP SERVER] Error resetting stats: {e}");
                            SteamResponse::Error(e)
                        }
                    };
                send_response(parent_tx, &response);
            }

            SteamCommand::UnlockAllAchievements(app_id_param) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<bool> = match app_manager.unlock_all_achievements() {
                    Ok(_) => SteamResponse::Success(true),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error unlocking all achievements: {e}");
                        SteamResponse::Error(e)
                    }
                };
                send_response(parent_tx, &response);
            }

            SteamCommand::ExportAppProgress(app_id_param) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let response: SteamResponse<AppExport> =
                    match collect_app_export(app_manager, app_id) {
                        Ok(export) => SteamResponse::Success(export),
                        Err(e) => SteamResponse::Error(e),
                    };
                send_response(parent_tx, &response);
            }

            SteamCommand::ImportAppProgress(app_id_param, payload) => {
                if !check_app_id(app_id_param, app_id, parent_tx) {
                    continue;
                }
                let summary = apply_app_export(app_manager, payload);
                send_response(parent_tx, &SteamResponse::Success(summary));
            }

            _ => {
                dev_println!("[APP SERVER] Received unknown command {command:?}");
                send_response::<()>(parent_tx, &SteamResponse::Error(SamError::UnknownError));
            }
        }
    }

    dev_println!("[APP SERVER] Exiting");

    0
}
