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

#[derive(Debug, PartialEq)]
pub enum SteamClientError {
    NullVtable,
    PipeCreationFailed,
    PipeReleaseFailed,
    UserConnectionFailed,
    InterfaceCreationFailed(String),
    AppNotFound,
    UnknownError,
}

impl std::fmt::Display for SteamClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamClientError::NullVtable => write!(f, "Steam client vtable is null"),
            SteamClientError::PipeCreationFailed => write!(f, "Failed to create steam pipe"),
            SteamClientError::PipeReleaseFailed => write!(f, "Failed to release steam pipe"),
            SteamClientError::UserConnectionFailed => {
                write!(f, "Failed to connect to steam server")
            }
            SteamClientError::InterfaceCreationFailed(name) => {
                write!(f, "Failed to create steam interface: {}", name)
            }
            SteamClientError::AppNotFound => write!(f, "App not found"),
            SteamClientError::UnknownError => write!(f, "Unknown Steam error"),
        }
    }
}

impl std::error::Error for SteamClientError {}

pub enum SteamCallbackId {
    GlobalAchievementPercentagesReady = 1110,
}
