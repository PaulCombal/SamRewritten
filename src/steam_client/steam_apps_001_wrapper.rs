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

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use crate::steam_client::steam_apps_001_vtable::ISteamApps001;
use crate::steam_client::wrapper_types::SteamError;

/// Safe wrapper for ISteamApps
pub struct SteamApps001 {
    inner: Arc<SteamApps001Inner>,
}

struct SteamApps001Inner {
    ptr: *mut ISteamApps001,
}

#[allow(dead_code)]
pub enum SteamApps001AppDataKeys<'a> {
    Name,
    Logo,
    SmallCapsule(&'a str),
    MetacriticScore,
    Developer
}

impl<'a> SteamApps001AppDataKeys<'a> {
    pub fn as_string(&self) -> String {
        match self {
            SteamApps001AppDataKeys::Name => "name\0".to_string(),
            SteamApps001AppDataKeys::SmallCapsule(language) => format!("small_capsule/{language}\0"),
            SteamApps001AppDataKeys::Logo => "logo\0".to_string(),
            SteamApps001AppDataKeys::MetacriticScore => "metacritic_score\0".to_string(),
            SteamApps001AppDataKeys::Developer => "developer\0".to_string(),
        }
    }
}

impl SteamApps001 {
    /// Creates a new SteamApps instance from a raw pointer
    /// # Safety
    /// The pointer must be valid and remain valid for the lifetime of the SteamApps
    pub unsafe fn from_raw(ptr: *mut ISteamApps001) -> Self {
        Self {
            inner: Arc::new(SteamApps001Inner { ptr }),
        }
    }

    /// TODO: document
    ///
    pub fn get_app_data(&self, app_id: &u32, key: &str) -> Result<String, SteamError> {
        let mut buffer = vec![0u8; 256];

        unsafe {
            // Get the vtable - return error if null
            let vtable = (*self.inner.ptr).vtable.as_ref()
                .ok_or(SteamError::NullVtable)?;

            // Call through the vtable
            let result = (vtable.get_app_data)(
                self.inner.ptr,
                *app_id,
                key.as_ptr() as *const c_char,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as c_int
            );
            
            if result == 0 {
                return Err(SteamError::UnknownError);
            }

            let c_str = CStr::from_ptr(buffer.as_ptr() as *const c_char);
            Ok(c_str.to_string_lossy().into_owned())
        }
    }
}
