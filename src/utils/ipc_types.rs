use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
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
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SteamResponse<T> {
    Success(T),
    Error(String),
    Pending,
}