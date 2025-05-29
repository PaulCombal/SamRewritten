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
