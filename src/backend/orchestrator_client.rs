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
use crate::backend::user_unlock_times::{AchievementUnlock, AvatarImage, Friend};
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

/// Declares a request type: the struct, its `Request` impl (response type),
/// and the `From<X> for SteamCommand` mapping. Two forms — unit, and struct
/// with fields. The `=> SteamCommand::...` expression sees the struct's fields
/// as bindings, so it can reorder them when the wire variant's tuple shape
/// differs from the struct's field order (see `SetAchievement`).
macro_rules! request {
    ($name:ident -> $resp:ty => $variant:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name;
        impl Request for $name {
            type Response = $resp;
        }
        impl From<$name> for SteamCommand {
            fn from(_: $name) -> Self {
                $variant
            }
        }
    };
    ($name:ident { $($field:ident : $ty:ty),* $(,)? } -> $resp:ty => $variant:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
        impl Request for $name {
            type Response = $resp;
        }
        impl From<$name> for SteamCommand {
            fn from(val: $name) -> Self {
                let $name { $($field,)* } = val;
                $variant
            }
        }
    };
}

request!(Shutdown -> bool => SteamCommand::Shutdown);
request!(GetRunningApps -> Vec<u32> => SteamCommand::GetRunningApps);

request!(GetSubscribedAppList { include_playtime: bool, with_achievement_counts: bool }
    -> Vec<AppModel>
    => SteamCommand::GetSubscribedAppList(include_playtime, with_achievement_counts));

request!(LaunchApp { app_id: u32 } -> bool => SteamCommand::LaunchApp(app_id));
request!(StopApp { app_id: u32 } -> bool => SteamCommand::StopApp(app_id));

request!(SetAchievement { app_id: u32, achievement_id: String, unlocked: bool, store: bool }
    -> bool
    => SteamCommand::SetAchievement(app_id, unlocked, achievement_id, store));

request!(StoreStatsAndAchievements { app_id: u32 } -> bool
    => SteamCommand::StoreStatsAndAchievements(app_id));
request!(UnlockAllAchievements { app_id: u32 } -> bool
    => SteamCommand::UnlockAllAchievements(app_id));

request!(SetIntStat { app_id: u32, stat_id: String, value: i32 } -> bool
    => SteamCommand::SetIntStat(app_id, stat_id, value));
request!(SetFloatStat { app_id: u32, stat_id: String, value: f32 } -> bool
    => SteamCommand::SetFloatStat(app_id, stat_id, value));

request!(ResetStats { app_id: u32, achievements_too: bool } -> bool
    => SteamCommand::ResetStats(app_id, achievements_too));

request!(GetAchievementCounts { app_ids: Vec<u32> } -> Vec<(u32, u32, u32)>
    => SteamCommand::GetAchievementCounts(app_ids));

request!(GetAchievementsAndStats { app_id: u32, launch: bool } -> AppProgress
    => SteamCommand::GetAchievementsAndStats(app_id, launch));

request!(GetFriendUnlockTimes { app_id: u32, friend: String } -> Vec<AchievementUnlock>
    => SteamCommand::GetFriendUnlockTimes(app_id, friend));

request!(GetFriendAchievementCount { app_id: u32, steam_id64: u64 } -> (u32, u32)
    => SteamCommand::GetFriendAchievementCount(app_id, steam_id64));

request!(GetFriends -> Vec<Friend> => SteamCommand::GetFriends);

request!(GetUserAvatar { steam_id64: u64 } -> Option<AvatarImage>
    => SteamCommand::GetUserAvatar(steam_id64));

request!(GetUserPersonaName { steam_id64: u64 } -> Option<String>
    => SteamCommand::GetUserPersonaName(steam_id64));

request!(ExportApps { app_ids: Vec<u32> } -> Vec<(u32, Result<AppExport, SamError>)>
    => SteamCommand::ExportApps(app_ids));
request!(ImportApps { apps: Vec<AppExport> } -> Vec<(u32, Result<ImportSummary, SamError>)>
    => SteamCommand::ImportApps(apps));
request!(UnlockAllApps { app_ids: Vec<u32> } -> Vec<(u32, Result<bool, SamError>)>
    => SteamCommand::UnlockAllApps(app_ids));
request!(ResetApps { app_ids: Vec<u32>, achievements_too: bool } -> Vec<(u32, Result<bool, SamError>)>
    => SteamCommand::ResetApps(app_ids, achievements_too));
