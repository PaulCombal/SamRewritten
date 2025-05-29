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

#[derive(Debug)]
pub enum SteamError {
    NullVtable,
    PipeCreationFailed,
    PipeReleaseFailed,
    UserConnectionFailed,
    InterfaceCreationFailed(String),
    AppNotFound,
    UnknownError,
}

impl std::fmt::Display for SteamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamError::NullVtable => write!(f, "Steam client vtable is null"),
            SteamError::PipeCreationFailed => write!(f, "Failed to create steam pipe"),
            SteamError::PipeReleaseFailed => write!(f, "Failed to release steam pipe"),
            SteamError::UserConnectionFailed => write!(f, "Failed to connect to steam server"),
            SteamError::InterfaceCreationFailed(name) => write!(f, "Failed to create steam interface: {}", name),
            SteamError::AppNotFound => write!(f, "App not found"),
            SteamError::UnknownError => write!(f, "Unknown Steam error"),
        }
    }
}

impl std::error::Error for SteamError {}

pub enum SteamCallbackId {
    GlobalAchievementPercentagesReady = 1110,
}
