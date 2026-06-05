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
use crate::steam_client::steamworks_types::CSteamID;
use crate::utils::ipc_types::SamError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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
    /// Steam avatar CDN url, or empty if the friend has no custom avatar.
    pub avatar_url: String,
}

/// Raw RGBA avatar pixels fetched natively from Steam (`GetSmallFriendAvatar` +
/// `GetImageRGBA`), for SteamIDs that have no cached CDN url (e.g. a pasted
/// custom SteamID). Travels over IPC to the frontend, which builds a texture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The `friends` block of `localconfig.vdf`, read as a loose value map: it mixes
/// scalar settings with `<id> { "name" ... }` objects, so callers keep the object
/// entries and ignore the rest.
#[derive(Deserialize)]
struct FriendsConfig {
    #[serde(default)]
    friends: HashMap<String, serde_json::Value>,
}

impl FriendsConfig {
    fn load(localconfig: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(localconfig).ok()?;
        keyvalues_serde::from_str(&contents).ok()
    }
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
        match friends.get_small_friend_avatar(steam_id) {
            Ok(h) if h > 0 => {
                handle = h;
                break;
            }
            Ok(0) if !still_fetching => return Ok(None),
            Ok(_) => {}
            Err(e) => {
                eprintln!("[USER UNLOCK TIMES] get_small_friend_avatar error: {e}");
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

/// All individual friends from the current user's `localconfig.vdf` friends block,
/// sorted by name. Clans/groups (stored as full SteamID64 keys) and scalar
/// settings entries are skipped; the avatar hash becomes a CDN url.
pub fn list_friends(localconfig: &Path) -> Vec<Friend> {
    let Some(root) = FriendsConfig::load(localconfig) else {
        return Vec::new();
    };

    let mut out: Vec<Friend> = Vec::new();
    for (key, val) in &root.friends {
        let Some(obj) = val.as_object() else {
            continue;
        };
        let Some(name) = obj.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        // Individual friends are stored by 32-bit account id; larger keys are
        // clans/groups, which have no per-game achievement stats.
        let Ok(account_id) = key.parse::<u64>() else {
            continue;
        };
        if account_id >= (1u64 << 32) {
            continue;
        }
        let avatar = obj.get("avatar").and_then(|a| a.as_str()).unwrap_or("");
        let avatar_url = if avatar.len() == 40 {
            format!("https://avatars.steamstatic.com/{avatar}_full.jpg")
        } else {
            String::new()
        };
        out.push(Friend {
            name: name.to_string(),
            steam_id64: account_id + STEAMID64_BASE,
            avatar_url,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Resolve a friend's persona name to their SteamID64 from the `friends` block of
/// the current user's `localconfig.vdf`. Keys are 32-bit account ids or full
/// SteamID64s. Case-insensitive.
pub fn find_friend_steamid64(localconfig: &Path, persona: &str) -> Option<u64> {
    let root = FriendsConfig::load(localconfig)?;

    for (key, val) in &root.friends {
        let Some(obj) = val.as_object() else {
            continue;
        };
        let matches = obj
            .get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.eq_ignore_ascii_case(persona))
            .unwrap_or(false);
        if matches {
            let id = key.parse::<u64>().ok()?;
            return Some(if id < (1u64 << 32) {
                id + STEAMID64_BASE
            } else {
                id
            });
        }
    }
    None
}
