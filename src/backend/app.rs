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

use crate::backend::app_manager::AppManager;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::ipc_types::{SamError, SamSerializable, SteamCommand, SteamResponse};
use interprocess::unnamed_pipe::{Recver, Sender};
use std::io::Write;

pub fn app(app_id: AppId_t, parent_tx: &mut Sender, parent_rx: &mut Recver) -> i32 {
    let mut app_manager = AppManager::new_connected(app_id);

    #[cfg(debug_assertions)]
    if app_manager.as_ref().is_err() {
        dev_println!("[APP SERVER] Failed to connect to Steam");
    }

    loop {
        dev_println!("[APP SERVER] Main loop...");

        let command =
            SteamCommand::from_recver(parent_rx).expect("[APP SERVER] No message from pipe");

        if app_manager.as_ref().is_err() {
            let response: SteamResponse<()> = SteamResponse::Error(SamError::SteamConnectionFailed);
            let response = response.sam_serialize();
            parent_tx
                .write_all(&response)
                .expect("[APP SERVER] Failed to send response");
            continue;
        }

        let app_manager = app_manager.as_mut().unwrap();

        match command {
            SteamCommand::Status => {
                let response = SteamResponse::Success(true).sam_serialize();
                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::Shutdown => {
                let response = SteamResponse::Success(true).sam_serialize();
                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
                break;
            }

            SteamCommand::GetAchievements(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.get_achievements() {
                    Ok(achievements) => SteamResponse::Success(achievements),
                    Err(e) => SteamResponse::Error::<Vec<AchievementInfo>>(e),
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::GetStats(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.get_statistics() {
                    Ok(statistics) => SteamResponse::Success(statistics),
                    Err(e) => SteamResponse::Error::<Vec<StatInfo>>(e),
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::SetAchievement(app_id_param, unlocked, achievement_id, store) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.set_achievement(&achievement_id, unlocked, store) {
                    Ok(_) => SteamResponse::Success(true),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting achievement: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::SetIntStat(app_id_param, stat_id, value) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.set_stat_i32(&stat_id, value) {
                    Ok(result) => SteamResponse::Success(result),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting int stat: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::SetFloatStat(app_id_param, stat_id, value) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.set_stat_f32(&stat_id, value) {
                    Ok(result) => SteamResponse::Success(result),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error setting float stat: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::StoreStatsAndAchievements(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.store_stats_and_achievements() {
                    Ok(_) => SteamResponse::Success(true),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error storing stats and achievements: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::ResetStats(app_id_param, achievements_too) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.reset_all_stats(achievements_too) {
                    Ok(result) => SteamResponse::Success(result),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error resetting stats: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            SteamCommand::UnlockAllAchievements(app_id_param) => {
                if app_id_param != app_id {
                    dev_println!("[APP SERVER] App ID mismatch: {app_id_param} != {app_id}");
                    let response =
                        SteamResponse::<()>::Error(SamError::AppMismatchError).sam_serialize();
                    parent_tx
                        .write_all(&response)
                        .expect("[APP SERVER] Failed to send response");
                    continue;
                }

                let response = match app_manager.unlock_all_achievements() {
                    Ok(_) => SteamResponse::Success(true),
                    Err(e) => {
                        dev_println!("[APP SERVER] Error unlocking all achievements: {e}");
                        SteamResponse::Error::<bool>(e)
                    }
                };
                let response = response.sam_serialize();

                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }

            _ => {
                dev_println!("[APP SERVER] Received unknown command {command:?}");
                let response = SteamResponse::<()>::Error(SamError::UnknownError).sam_serialize();
                parent_tx
                    .write_all(&response)
                    .expect("[APP SERVER] Failed to send response");
            }
        }
    }

    dev_println!("[APP SERVER] Exiting");

    0
}
