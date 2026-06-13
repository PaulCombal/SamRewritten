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

use crate::steam_client::steam_friends_vtable::ISteamFriends;
use crate::steam_client::steamworks_types::CSteamID;
use crate::steam_client::wrapper_types::SteamClientError;
use std::ffi::CStr;
use std::sync::Arc;

pub struct SteamFriends {
    inner: Arc<SteamFriendsInner>,
}

struct SteamFriendsInner {
    ptr: *mut ISteamFriends,
}

impl SteamFriends {
    pub unsafe fn from_raw(ptr: *mut ISteamFriends) -> Self {
        Self {
            inner: Arc::new(SteamFriendsInner { ptr }),
        }
    }

    pub fn get_small_friend_avatar(&self, steam_id: CSteamID) -> Result<i32, SteamClientError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            Ok((vtable.get_small_friend_avatar)(self.inner.ptr, steam_id))
        }
    }

    pub fn request_user_information(
        &self,
        steam_id: CSteamID,
        name_only: bool,
    ) -> Result<bool, SteamClientError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            Ok((vtable.request_user_information)(
                self.inner.ptr,
                steam_id,
                name_only,
            ))
        }
    }

    pub fn get_friend_persona_name(&self, steam_id: CSteamID) -> Result<String, SteamClientError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let ptr = (vtable.get_friend_persona_name)(self.inner.ptr, steam_id);
            if ptr.is_null() {
                return Ok(String::new());
            }
            Ok(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }
}
