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

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SamError {
    SerializationFailed,
    SteamConnectionFailed,
    AppListRetrievalFailed,
    SocketCommunicationFailed,
    StatStoreFailed,
    LockUnlockAchievementFailed,
    AppMismatchError,
    Timeout,
    UnknownError,
}

impl std::fmt::Display for SamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SamError::SerializationFailed => write!(f, "SAM: Serialization failed"),
            SamError::SteamConnectionFailed => write!(f, "SAM: Steam connection failed"),
            SamError::AppListRetrievalFailed => write!(f, "SAM: App list retrieval failed"),
            SamError::UnknownError => write!(f, "SAM: Unknown error"),
            SamError::SocketCommunicationFailed => {
                write!(f, "SAM: SocketCommunication failed")
            }
            SamError::AppMismatchError => write!(f, "SAM: App mismatch"),
            SamError::StatStoreFailed => write!(f, "SAM: Stat/ach store failed"),
            SamError::LockUnlockAchievementFailed => {
                write!(f, "SAM: Lock/unlock achievement failed")
            }
            SamError::Timeout => write!(f, "SAM: Steam is busy, try again with a smaller batch"),
        }
    }
}

impl std::error::Error for SamError {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppAchievementExport {
    pub id: String,
    pub is_achieved: bool,
    pub permission: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AppStatValue {
    Int(i32),
    Float(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppStatExport {
    pub id: String,
    pub value: AppStatValue,
    pub permission: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppExport {
    pub app_id: u32,
    #[serde(default)]
    pub app_name: String,
    pub achievements: Vec<AppAchievementExport>,
    pub stats: Vec<AppStatExport>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ImportSummary {
    pub achievements_applied: usize,
    pub stats_applied: usize,
    pub skipped_protected: Vec<String>,
    pub skipped_unwriteable: Vec<String>,
    pub errors: Vec<String>,
    pub reset_would_help: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SteamCommand {
    /// `(include_playtime, with_achievement_counts)`. When `with_achievement_counts`
    /// is true, each returned `AppModel` will have `achievement_count` and
    /// `unlocked_achievement_count` populated for apps whose schema is cached locally.
    GetSubscribedAppList(bool, bool),
    LaunchApp(u32),
    StopApp(u32),
    StopApps,
    GetRunningApps,
    Shutdown,
    Status, // Ask for status of the process
    GetAchievements(u32),
    GetStats(u32),
    SetAchievement(u32, bool, String, bool),
    SetIntStat(u32, String, i32),
    SetFloatStat(u32, String, f32),
    ResetStats(u32, bool),
    UnlockAllAchievements(u32),
    StoreStatsAndAchievements(u32),
    ExportAppProgress(u32),
    ImportAppProgress(u32, AppExport),
    GetAchievementCounts(Vec<u32>),
    /// Fetch `app_id`'s achievements and stats in a single round-trip, so an
    /// unrelated batch command can't interleave between the two fetches on the
    /// serial channel. When `launch` is true the app is launched (or its
    /// refcount bumped) first; otherwise it must already be running. Returns
    /// `(achievements, stats)`.
    GetAchievementsAndStats(u32, bool),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SteamResponse<T> {
    Success(T),
    Error(SamError),
}

/// Serialize a message as a length-prefixed (`usize` little-endian) JSON frame.
pub fn frame_message<T: Serialize + ?Sized>(msg: &T) -> Vec<u8> {
    let serialized = serde_json::to_vec(msg).expect("Serializing IPC message must not fail");
    let length_bytes = serialized.len().to_le_bytes();
    let mut frame = Vec::with_capacity(length_bytes.len() + serialized.len());
    frame.extend_from_slice(&length_bytes);
    frame.extend_from_slice(&serialized);
    frame
}

/// Frame `msg` and write it to `w`. Used by both ends of the pipe.
pub fn write_message<T: Serialize + ?Sized>(w: &mut impl Write, msg: &T) -> Result<(), SamError> {
    let frame = frame_message(msg);
    w.write_all(&frame).map_err(|e| {
        eprintln!("[IPC] Failed to write framed message: {e}");
        SamError::SocketCommunicationFailed
    })
}

/// Read a length-prefixed JSON frame and return the JSON payload (no prefix).
pub fn read_frame(r: &mut impl Read) -> Result<Vec<u8>, SamError> {
    let mut len_buf = [0u8; size_of::<usize>()];
    r.read_exact(&mut len_buf).map_err(|e| {
        eprintln!("[IPC] Failed to read message length: {e}");
        SamError::SocketCommunicationFailed
    })?;
    let data_len = usize::from_le_bytes(len_buf);
    let mut payload = vec![0u8; data_len];
    r.read_exact(&mut payload).map_err(|e| {
        eprintln!("[IPC] Failed to read message payload: {e}");
        SamError::SocketCommunicationFailed
    })?;
    Ok(payload)
}

/// Read a framed message and deserialize the JSON payload.
pub fn read_message<T: DeserializeOwned>(r: &mut impl Read) -> Result<T, SamError> {
    let payload = read_frame(r)?;
    serde_json::from_slice(&payload).map_err(|e| {
        eprintln!("[IPC] Failed to deserialize message: {e}");
        SamError::SerializationFailed
    })
}

/// Read a framed message and return its bytes *with the length prefix intact*.
/// Used by the orchestrator to proxy a child's response to the parent verbatim.
pub fn read_frame_raw(r: &mut impl Read) -> Result<Vec<u8>, SamError> {
    let payload = read_frame(r)?;
    let mut frame = Vec::with_capacity(size_of::<usize>() + payload.len());
    frame.extend_from_slice(&payload.len().to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Parse a framed `SteamResponse<T>` payload (length prefix + JSON) into a
/// `Result<T, SamError>`. The input may be either with or without the
/// `usize` length prefix.
pub fn parse_response_bytes<T: DeserializeOwned>(framed: &[u8]) -> Result<T, SamError> {
    let len_size = size_of::<usize>();
    let json_bytes = if framed.len() >= len_size {
        &framed[len_size..]
    } else {
        framed
    };
    let response: SteamResponse<T> = serde_json::from_slice(json_bytes).map_err(|e| {
        eprintln!("[IPC] Failed to parse response: {e}");
        SamError::SerializationFailed
    })?;
    response.into()
}

impl<T> From<SteamResponse<T>> for Result<T, SamError> {
    fn from(val: SteamResponse<T>) -> Self {
        match val {
            SteamResponse::Success(data) => Ok(data),
            SteamResponse::Error(error) => Err(error),
        }
    }
}
