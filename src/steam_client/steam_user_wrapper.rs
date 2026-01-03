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

use crate::steam_client::steam_user_vtable::ISteamUser;
use crate::steam_client::steamworks_types::CSteamID;
use crate::steam_client::wrapper_types::SteamClientError;
use std::sync::Arc;

pub struct SteamUser {
    inner: Arc<SteamUserInner>,
}

struct SteamUserInner {
    ptr: *mut ISteamUser,
}

impl SteamUser {
    pub unsafe fn from_raw(ptr: *mut ISteamUser) -> Self {
        Self {
            inner: Arc::new(SteamUserInner { ptr }),
        }
    }

    #[cfg(unix)]
    pub fn get_steam_id(&self) -> Result<CSteamID, SteamClientError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;

            let steam_id = (vtable.get_steam_id)(self.inner.ptr);
            Ok(steam_id)
        }
    }

    #[cfg(windows)]
    pub fn get_steam_id(&self) -> Result<CSteamID, SteamClientError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;

            let mut id64 = 0u64;
            (vtable.get_steam_id)(self.inner.ptr, &mut id64);
            let steam_id = CSteamID { m_steamid: id64 };

            Ok(steam_id)
        }
    }
}
