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

//! Native friends-interface queries (avatar, persona name) and the
//! `localconfig.vdf` friends-block readers that back copy-timing mode.

use crate::steam_client::steam_friends_wrapper::SteamFriends;
use crate::steam_client::steam_utils_wrapper::SteamUtils;
use crate::steam_client::steamworks_types::{CSteamID, K_E_FRIEND_FLAG_IMMEDIATE};
use crate::utils::ipc_types::SamError;
use serde::{Deserialize, Serialize};

/// SteamID64 of the first individual account. `account_id` (SteamID3) + this base
/// = SteamID64; subtracting recovers the account id used in cache filenames.
pub const STEAMID64_BASE: u64 = 76561197960265728;

/// The 32-bit account id (low 32 bits of a SteamID64), as used in cache filenames
/// and `userdata/` paths. Inverse of `account_id + STEAMID64_BASE`.
pub fn account_id(steam_id64: u64) -> u32 {
    (steam_id64 & 0xFFFF_FFFF) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub name: String,
    pub steam_id64: u64,
    /// Optional avatar CDN url. Always empty for natively-enumerated friends
    /// (their avatars load on demand as RGBA via `fetch_user_avatar`); kept so a
    /// caller that already has a url can pass it straight to the frontend.
    pub avatar_url: String,
}

/// Raw RGBA avatar pixels fetched natively from Steam (`GetMediumFriendAvatar` +
/// `GetImageRGBA`). Travels over IPC to the frontend, which builds a texture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Fetch a user's avatar as raw RGBA via Steam's global friends interface.
/// `None` if the user has no avatar or it never loads in time.
pub fn fetch_user_avatar(
    friends: &SteamFriends,
    utils: &SteamUtils,
    steam_id64: u64,
) -> Result<Option<AvatarImage>, SamError> {
    let steam_id = CSteamID {
        m_steamid: steam_id64,
    };

    // false => we already have everything, so a 0 handle below means "no avatar"
    // rather than "not downloaded yet".
    let still_fetching = friends
        .request_user_information(steam_id, false)
        .unwrap_or(true);

    // Handle: >0 ready, -1 downloading, 0 none/unknown. Poll ~5s for a cold user.
    let mut handle = 0;
    for _ in 0..200 {
        match friends.get_medium_friend_avatar(steam_id) {
            Ok(h) if h > 0 => {
                handle = h;
                break;
            }
            Ok(0) if !still_fetching => return Ok(None),
            Ok(_) => {}
            Err(e) => {
                eprintln!("[USER UNLOCK TIMES] get_medium_friend_avatar error: {e}");
                return Err(SamError::UnknownError);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if handle <= 0 {
        return Ok(None);
    }

    let (width, height) = utils.get_image_size(handle).map_err(|e| {
        eprintln!("[USER UNLOCK TIMES] get_image_size error: {e}");
        SamError::UnknownError
    })?;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    utils.get_image_rgba(handle, &mut rgba).map_err(|e| {
        eprintln!("[USER UNLOCK TIMES] get_image_rgba error: {e}");
        SamError::UnknownError
    })?;
    Ok(Some(AvatarImage {
        width,
        height,
        rgba,
    }))
}

/// Resolve a user's persona (display) name natively from Steam, downloading it
/// first for a non-friend. `None` if it never loads.
pub fn fetch_user_persona_name(
    friends: &SteamFriends,
    steam_id64: u64,
) -> Result<Option<String>, SamError> {
    let steam_id = CSteamID {
        m_steamid: steam_id64,
    };
    let still_fetching = friends
        .request_user_information(steam_id, true)
        .unwrap_or(true);

    for _ in 0..200 {
        match friends.get_friend_persona_name(steam_id) {
            Ok(name) if !name.is_empty() && name != "[unknown]" => {
                return Ok(Some(name));
            }
            Ok(_) if !still_fetching => return Ok(None),
            Ok(_) => {}
            Err(e) => {
                eprintln!("[USER UNLOCK TIMES] get_friend_persona_name error: {e}");
                return Err(SamError::UnknownError);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    Ok(None)
}

/// SteamIDs of the current user's confirmed friends, in Steam's enumeration
/// order. Only `k_EFriendFlagImmediate` is requested, so removed/blocked/ignored
/// users and pending requests are excluded.
fn immediate_friend_ids(friends: &SteamFriends) -> Vec<CSteamID> {
    let count = friends
        .get_friend_count(K_E_FRIEND_FLAG_IMMEDIATE)
        .unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            friends
                .get_friend_by_index(i, K_E_FRIEND_FLAG_IMMEDIATE)
                .ok()
        })
        .filter(|id| id.m_steamid != 0)
        .collect()
}

/// The current user's confirmed friends, enumerated live from Steam and sorted by
/// name. Avatars load on demand (see `fetch_user_avatar`), so `avatar_url` is left
/// empty.
pub fn list_friends(friends: &SteamFriends) -> Vec<Friend> {
    let mut out: Vec<Friend> = immediate_friend_ids(friends)
        .into_iter()
        .map(|steam_id| Friend {
            name: friends
                .get_friend_persona_name(steam_id)
                .unwrap_or_default(),
            steam_id64: steam_id.m_steamid,
            avatar_url: String::new(),
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Resolve a confirmed friend's persona name to their SteamID64 by matching
/// against the live friends list (case-insensitive). `None` if no friend matches.
pub fn find_friend_steamid64(friends: &SteamFriends, persona: &str) -> Option<u64> {
    immediate_friend_ids(friends)
        .into_iter()
        .find(|steam_id| {
            friends
                .get_friend_persona_name(*steam_id)
                .map(|n| n.eq_ignore_ascii_case(persona))
                .unwrap_or(false)
        })
        .map(|steam_id| steam_id.m_steamid)
}
