use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum SteamCommand {
    GetOwnedAppList,
    LaunchApp(u32),
    StopApp(u32),
    StopApps,
    Shutdown,
    Status, // Ask for status of the process
    GetAchievements(u32),
    GetStats(u32),
    SetAchievement(u32, bool, String),
    SetIntStat(u32, String, i32),
    SetFloatStat(u32, String, f32),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SteamResponse<T> {
    Success(T),
    Error(String)
}

impl<T> Into<Result<T, String>> for SteamResponse<T> {
    fn into(self) -> Result<T, String> {
        match self {
            SteamResponse::Success(data) => Ok(data),
            SteamResponse::Error(error) => Err(error),
        }
    }
}