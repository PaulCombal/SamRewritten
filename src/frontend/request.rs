use std::fmt::Debug;
use std::io::{BufRead, BufReader, Write};
use interprocess::local_socket::traits::Stream;
use serde::de::DeserializeOwned;
use crate::backend::app_lister::AppModel;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::utils::ipc_types::{SteamCommand, SteamResponse};
use super::ipc_process::get_orchestrator_socket_path;
use interprocess::local_socket::prelude::LocalSocketStream;

pub trait Request: Into<SteamCommand> + Debug + Clone {
    type Response: DeserializeOwned;

    fn request(self) -> Option<Self::Response> {
        println!("[CLIENT] Requesting {self:?}");
        let (_, socket_name) = get_orchestrator_socket_path();
        let mut stream = LocalSocketStream::connect(socket_name)
            .inspect_err(|error| eprintln!("[CLIENT] Request stream failed: {error}"))
            .ok()?;

        let command = self.clone().into();
        serde_json::to_writer(&mut stream, &command)
            .inspect_err(|error| eprintln!("[CLIENT] Request serialization failed: {error}"))
            .ok()?;

        stream.write_all(b"\n")
            .inspect_err(|error| eprintln!("[CLIENT] Request write failed: {error}"))
            .ok()?;

        stream.flush()
            .inspect_err(|error| eprintln!("[CLIENT] Request flush failed: {error}"))
            .ok()?;

        let mut buffer = String::new();
        BufReader::new(stream).read_line(&mut buffer)
            .inspect_err(|error| eprintln!("[CLIENT] Response data read failed: {error}"))
            .ok()?;

        serde_json::from_str::<SteamResponse<Self::Response>>(buffer.as_str())
            .map(|response| Into::<Result<Self::Response, String>>::into(response))
            .inspect_err(|error| eprintln!("[CLIENT] Response deserialization failed: {error}")).ok()?
            .inspect_err(|error| eprintln!("[CLIENT] Request failed: {error}"))
            .ok()
    }
}

#[derive(Debug, Clone)]
pub struct GetOwnedAppList;

#[derive(Debug, Clone)]
pub struct Shutdown;

#[derive(Debug, Clone)]
pub struct LaunchApp {
    pub app_id: u32
}

#[derive(Debug, Clone)]
pub struct StopApp {
    pub app_id: u32
}

#[derive(Debug, Clone)]
pub struct StopApps;

#[derive(Debug, Clone)]
pub struct GetAchievements {
    pub app_id: u32
}

#[derive(Debug, Clone)]
pub struct GetStats {
    pub app_id: u32
}

#[derive(Debug, Clone)]
pub struct SetAchievement {
    pub app_id: u32,
    pub achievement_id: String,
    pub unlocked: bool,
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

impl Request for GetOwnedAppList {
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

impl Request for StopApps {
    type Response = bool;
}

impl Request for GetAchievements {
    type Response = Vec<AchievementInfo>;
}

impl Request for GetStats {
    type Response = Vec<StatInfo>;
}

impl Request for SetAchievement {
    type Response = bool;
}

impl Request for SetIntStat {
    type Response = bool;
}

impl Request for SetFloatStat {
    type Response = bool;
}

impl Into<SteamCommand> for GetOwnedAppList {
    fn into(self) -> SteamCommand {
        SteamCommand::GetOwnedAppList
    }
}

impl Into<SteamCommand> for Shutdown {
    fn into(self) -> SteamCommand {
        SteamCommand::Shutdown
    }
}

impl Into<SteamCommand> for LaunchApp {
    fn into(self) -> SteamCommand {
        SteamCommand::LaunchApp(self.app_id)
    }
}

impl Into<SteamCommand> for StopApp {
    fn into(self) -> SteamCommand {
        SteamCommand::StopApp(self.app_id)
    }
}

impl Into<SteamCommand> for StopApps {
    fn into(self) -> SteamCommand {
        SteamCommand::StopApps
    }
}

impl Into<SteamCommand> for GetAchievements {
    fn into(self) -> SteamCommand {
        SteamCommand::GetAchievements(self.app_id)
    }
}

impl Into<SteamCommand> for GetStats {
    fn into(self) -> SteamCommand {
        SteamCommand::GetStats(self.app_id)
    }
}

impl Into<SteamCommand> for SetAchievement {
    fn into(self) -> SteamCommand {
        SteamCommand::SetAchievement(self.app_id, self.unlocked, self.achievement_id)
    }
}

impl Into<SteamCommand> for SetIntStat {
    fn into(self) -> SteamCommand {
        SteamCommand::SetIntStat(self.app_id, self.stat_id, self.value)
    }
}

impl Into<SteamCommand> for SetFloatStat {
    fn into(self) -> SteamCommand {
        SteamCommand::SetFloatStat(self.app_id, self.stat_id, self.value)
    }
}