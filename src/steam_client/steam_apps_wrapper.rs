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

use crate::steam_client::steam_apps_vtable::ISteamApps;
use crate::steam_client::wrapper_types::SteamError;
use std::sync::Arc;

pub struct SteamApps {
    inner: Arc<SteamAppsInner>,
}

struct SteamAppsInner {
    ptr: *mut ISteamApps,
}

impl SteamApps {
    pub unsafe fn from_raw(ptr: *mut ISteamApps) -> Self {
        Self {
            inner: Arc::new(SteamAppsInner { ptr }),
        }
    }

    pub fn get_current_game_language(&self) -> String {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .expect("Null ISteamApps vtable");
            let lang_ptr = (vtable.get_current_game_language)(self.inner.ptr);
            std::ffi::CStr::from_ptr(lang_ptr)
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn is_subscribed_app(&self, app_id: u32) -> Result<bool, SteamError> {
        unsafe {
            // Get the vtable - return error if null
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            // Call through the vtable
            let is_subscribed = (vtable.b_is_subscribed_app)(self.inner.ptr, app_id);

            Ok(is_subscribed)
        }
    }
}
