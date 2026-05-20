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

use crate::backend::app_lister::AppModel;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
use crate::gui_frontend::DEFAULT_PROCESS;
use crate::utils::ipc_types::{SamError, SteamCommand};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// An app's achievements and stats, fetched together.
pub type AppProgress = (Vec<AchievementInfo>, Vec<StatInfo>);

pub trait Request: Into<SteamCommand> + Debug + Clone {
    type Response: DeserializeOwned;

    fn request(self) -> Result<Self::Response, SamError> {
        let mut guard = DEFAULT_PROCESS.lock().unwrap();
        let Some(ipc) = guard.as_mut() else {
            eprintln!("[CLIENT] No orchestrator process");
            return Err(SamError::SocketCommunicationFailed);
        };

        let command: SteamCommand = self.into();
        dev_println!("CLIENT", "Sending command: {:?}", command);
        ipc.request_response::<Self::Response, _>(&command)
    }
}

#[derive(Debug, Clone)]
pub struct GetSubscribedAppList {
    pub include_playtime: bool,
    pub with_achievement_counts: bool,
}

#[derive(Debug, Clone)]
pub struct Shutdown;

#[derive(Debug, Clone)]
pub struct LaunchApp {
    pub app_id: u32,
}

#[derive(Debug, Clone)]
pub struct StopApp {
    pub app_id: u32,
}

#[derive(Debug, Clone)]
pub struct GetRunningApps;

#[derive(Debug, Clone)]
pub struct SetAchievement {
    pub app_id: u32,
    pub achievement_id: String,
    pub unlocked: bool,
    pub store: bool,
}

#[derive(Debug, Clone)]
pub struct StoreStatsAndAchievements {
    pub app_id: u32,
}

#[derive(Debug, Clone)]
pub struct UnlockAllAchievements {
    pub app_id: u32,
}

#[derive(Debug, Clone)]
pub struct SetIntStat {
    pub app_id: u32,
    pub stat_id: String,
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct SetFloatStat {
    pub app_id: u32,
    pub stat_id: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct ResetStats {
    pub app_id: u32,
    pub achievements_too: bool,
}

#[derive(Debug, Clone)]
pub struct GetAchievementCounts {
    pub app_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct GetAchievementsAndStats {
    pub app_id: u32,
    pub launch: bool,
}

impl Request for GetSubscribedAppList {
    type Response = Vec<AppModel>;
}

impl Request for Shutdown {
    type Response = bool;
}

impl Request for LaunchApp {
    type Response = bool;
}

impl Request for StopApp {
    type Response = bool;
}

impl Request for GetRunningApps {
    type Response = Vec<u32>;
}

impl Request for SetAchievement {
    type Response = bool;
}

impl Request for UnlockAllAchievements {
    type Response = bool;
}

impl Request for StoreStatsAndAchievements {
    type Response = bool;
}

impl Request for SetIntStat {
    type Response = bool;
}

impl Request for SetFloatStat {
    type Response = bool;
}

impl Request for ResetStats {
    type Response = bool;
}

impl Request for GetAchievementCounts {
    type Response = Vec<(u32, u32, u32)>;
}

impl Request for GetAchievementsAndStats {
    type Response = AppProgress;
}

impl From<GetSubscribedAppList> for SteamCommand {
    fn from(val: GetSubscribedAppList) -> Self {
        SteamCommand::GetSubscribedAppList(val.include_playtime, val.with_achievement_counts)
    }
}

impl From<Shutdown> for SteamCommand {
    fn from(_val: Shutdown) -> Self {
        SteamCommand::Shutdown
    }
}

impl From<LaunchApp> for SteamCommand {
    fn from(val: LaunchApp) -> Self {
        SteamCommand::LaunchApp(val.app_id)
    }
}

impl From<StopApp> for SteamCommand {
    fn from(val: StopApp) -> Self {
        SteamCommand::StopApp(val.app_id)
    }
}

impl From<GetRunningApps> for SteamCommand {
    fn from(_val: GetRunningApps) -> Self {
        SteamCommand::GetRunningApps
    }
}

impl From<SetAchievement> for SteamCommand {
    fn from(val: SetAchievement) -> Self {
        SteamCommand::SetAchievement(val.app_id, val.unlocked, val.achievement_id, val.store)
    }
}

impl From<StoreStatsAndAchievements> for SteamCommand {
    fn from(val: StoreStatsAndAchievements) -> Self {
        SteamCommand::StoreStatsAndAchievements(val.app_id)
    }
}

impl From<UnlockAllAchievements> for SteamCommand {
    fn from(val: UnlockAllAchievements) -> Self {
        SteamCommand::UnlockAllAchievements(val.app_id)
    }
}

impl From<SetIntStat> for SteamCommand {
    fn from(val: SetIntStat) -> Self {
        SteamCommand::SetIntStat(val.app_id, val.stat_id, val.value)
    }
}

impl From<SetFloatStat> for SteamCommand {
    fn from(val: SetFloatStat) -> Self {
        SteamCommand::SetFloatStat(val.app_id, val.stat_id, val.value)
    }
}

impl From<ResetStats> for SteamCommand {
    fn from(val: ResetStats) -> Self {
        SteamCommand::ResetStats(val.app_id, val.achievements_too)
    }
}

impl From<GetAchievementCounts> for SteamCommand {
    fn from(val: GetAchievementCounts) -> Self {
        SteamCommand::GetAchievementCounts(val.app_ids)
    }
}

impl From<GetAchievementsAndStats> for SteamCommand {
    fn from(val: GetAchievementsAndStats) -> Self {
        SteamCommand::GetAchievementsAndStats(val.app_id, val.launch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_deserialization_debug() -> Result<(), SamError> {
        let message = r#"[{"mytest": 42}]"#;

        let _ = serde_json::from_str::<Vec<StatInfo>>(&message).map_err(|error| {
            error.column();
            eprintln!("[CLIENT] TEST Response deserialization failed: {error}");

            let column = error.column();
            let idx = column.saturating_sub(1);

            let start = idx.saturating_sub(3);
            let end = (idx + 3).min(message.len());

            let extract = &message[start..end];

            assert_eq!(extract, r#"est": "#);

            eprintln!("Message: {message}");
            eprintln!("Response type {}", std::any::type_name::<Vec<StatInfo>>());

            SamError::SocketCommunicationFailed
        });

        Ok(())
    }
}
