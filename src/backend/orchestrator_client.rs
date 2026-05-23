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

//! Frontend-agnostic client for talking to the orchestrator process. Both the
//! GUI and the CLI hold a single orchestrator over IPC and drive it through the
//! typed `Request` trait below, so neither frontend loads `steamclient.so`
//! itself — the orchestrator (and the children it spawns) own every Steam
//! connection.

use crate::backend::app_lister::AppModel;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::dev_println;
#[cfg(feature = "gui")]
use crate::utils::app_paths::get_executable_path;
#[cfg(feature = "gui")]
use crate::utils::bidir_child::BidirChild;
use crate::utils::ipc_client::IpcClient;
use crate::utils::ipc_types::{
    AppExport, ImportSummary, ProgressMsg, SamError, SteamCommand, SteamResponse,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
#[cfg(feature = "gui")]
use std::path::PathBuf;
#[cfg(feature = "gui")]
use std::process::Command;
use std::sync::Mutex;

/// The frontend's handle to the orchestrator IPC. Structurally a `Mutex` (not an
/// `RwLock`) because every caller needs `&mut IpcClient` to drive a request —
/// there are no concurrent readers to optimize for.
pub static ORCHESTRATOR: Mutex<Option<IpcClient>> = Mutex::new(None);

pub fn set_orchestrator(ipc: IpcClient) {
    *ORCHESTRATOR.lock().unwrap() = Some(ipc);
}

/// Spawn the orchestrator and install it as the global IPC handle. `chosen` pins
/// the Steam install via `SAM_STEAM_INSTALL_ROOT`; `None` uses the locator default.
#[cfg(feature = "gui")]
pub fn spawn_orchestrator(chosen: Option<PathBuf>) -> Result<(), SamError> {
    let mut command = Command::new(get_executable_path());
    command.arg("--orchestrator");
    if let Some(root) = chosen.as_ref() {
        command.env("SAM_STEAM_INSTALL_ROOT", root);
    }

    match BidirChild::new(&mut command) {
        Ok(child) => {
            set_orchestrator(IpcClient::new(child));
            Ok(())
        }
        Err(e) => {
            eprintln!("[CLIENT] Failed to spawn orchestrator: {e}");
            Err(SamError::SocketCommunicationFailed)
        }
    }
}

/// Tolerates an already-broken orchestrator pipe (e.g. Flatpak Steam quit and
/// took the namespace down with it) — errors are logged, not fatal.
pub fn shutdown_and_wait() {
    if let Err(err) = Shutdown.request() {
        eprintln!("[CLIENT] Failed to send shutdown message: {err}");
    }
    if let Some(ipc) = ORCHESTRATOR.lock().unwrap().as_mut() {
        if let Err(err) = ipc.wait() {
            eprintln!("[CLIENT] Failed to wait on orchestrator to shut down: {err}");
        }
    }
}

pub type AppProgress = (Vec<AchievementInfo>, Vec<StatInfo>);

pub trait Request: Into<SteamCommand> + Debug + Clone {
    type Response: DeserializeOwned;

    fn request(self) -> Result<Self::Response, SamError> {
        let mut guard = ORCHESTRATOR.lock().unwrap();
        let Some(ipc) = guard.as_mut() else {
            eprintln!("[CLIENT] No orchestrator process");
            return Err(SamError::SocketCommunicationFailed);
        };

        let command: SteamCommand = self.into();
        dev_println!("CLIENT", "Sending command: {:?}", command);
        ipc.request_response::<Self::Response, _>(&command)
    }

    /// Streaming variant for bulk fan-out commands: reads `ProgressMsg::Progress`
    /// frames into `on_progress(done, total)` until the terminal
    /// `ProgressMsg::Done(SteamResponse<Self::Response>)` arrives. Non-bulk
    /// commands should keep using `request()`
    fn request_with_progress<F>(self, mut on_progress: F) -> Result<Self::Response, SamError>
    where
        F: FnMut(usize, usize),
    {
        let mut guard = ORCHESTRATOR.lock().unwrap();
        let Some(ipc) = guard.as_mut() else {
            eprintln!("[CLIENT] No orchestrator process");
            return Err(SamError::SocketCommunicationFailed);
        };

        let command: SteamCommand = self.into();
        dev_println!("CLIENT", "Sending streaming command: {:?}", command);
        ipc.send(&command)?;
        loop {
            let msg: ProgressMsg<SteamResponse<Self::Response>> = ipc.recv()?;
            match msg {
                ProgressMsg::Progress { done, total } => on_progress(done, total),
                ProgressMsg::Done(resp) => return resp.into(),
            }
        }
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

#[derive(Debug, Clone)]
pub struct ExportApps {
    pub app_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct ImportApps {
    pub apps: Vec<AppExport>,
}

#[derive(Debug, Clone)]
pub struct UnlockAllApps {
    pub app_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct ResetApps {
    pub app_ids: Vec<u32>,
    pub achievements_too: bool,
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

impl Request for ExportApps {
    type Response = Vec<(u32, Result<AppExport, SamError>)>;
}

impl Request for ImportApps {
    type Response = Vec<(u32, Result<ImportSummary, SamError>)>;
}

impl Request for UnlockAllApps {
    type Response = Vec<(u32, Result<bool, SamError>)>;
}

impl Request for ResetApps {
    type Response = Vec<(u32, Result<bool, SamError>)>;
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

impl From<ExportApps> for SteamCommand {
    fn from(val: ExportApps) -> Self {
        SteamCommand::ExportApps(val.app_ids)
    }
}

impl From<ImportApps> for SteamCommand {
    fn from(val: ImportApps) -> Self {
        SteamCommand::ImportApps(val.apps)
    }
}

impl From<UnlockAllApps> for SteamCommand {
    fn from(val: UnlockAllApps) -> Self {
        SteamCommand::UnlockAllApps(val.app_ids)
    }
}

impl From<ResetApps> for SteamCommand {
    fn from(val: ResetApps) -> Self {
        SteamCommand::ResetApps(val.app_ids, val.achievements_too)
    }
}
