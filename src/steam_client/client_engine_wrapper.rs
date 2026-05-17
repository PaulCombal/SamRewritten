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

use crate::steam_client::client_engine_vtable::IClientEngine;
use crate::steam_client::client_user_stats_map_vtable::IClientUserStatsMap;
use crate::steam_client::client_user_stats_map_wrapper::ClientUserStatsMap;
use crate::steam_client::client_user_vtable::IClientUser;
use crate::steam_client::client_user_wrapper::ClientUser;
use crate::steam_client::steamworks_types::{HSteamPipe, HSteamUser};
use crate::steam_client::wrapper_types::SteamClientError;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClientEngine {
    inner: Arc<ClientEngineInner>,
}

pub(crate) struct ClientEngineInner {
    ptr: *mut IClientEngine,
}

impl ClientEngineInner {
    pub(crate) fn run_frame(&self) {
        unsafe {
            let vt = (*self.ptr).vtable.as_ref().expect("vtable null");
            (vt.run_frame)(self.ptr);
        }
    }
}

impl ClientEngine {
    pub unsafe fn from_raw(ptr: *mut IClientEngine) -> Self {
        Self {
            inner: Arc::new(ClientEngineInner { ptr }),
        }
    }

    pub fn create_steam_pipe(&self) -> Result<HSteamPipe, SteamClientError> {
        unsafe {
            let vt = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let pipe = (vt.create_steam_pipe)(self.inner.ptr);
            if pipe == 0 {
                Err(SteamClientError::PipeCreationFailed)
            } else {
                Ok(pipe)
            }
        }
    }

    pub fn release_steam_pipe(&self, pipe: HSteamPipe) -> Result<bool, SteamClientError> {
        unsafe {
            let vt = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let ok = (vt.release_steam_pipe)(self.inner.ptr, pipe);
            if ok {
                Ok(ok)
            } else {
                Err(SteamClientError::PipeReleaseFailed)
            }
        }
    }

    pub fn connect_to_global_user(&self, pipe: HSteamPipe) -> Result<HSteamUser, SteamClientError> {
        unsafe {
            let vt = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let user = (vt.connect_to_global_user)(self.inner.ptr, pipe);
            if user == 0 {
                Err(SteamClientError::UserConnectionFailed)
            } else {
                Ok(user)
            }
        }
    }

    pub fn release_user(&self, pipe: HSteamPipe, user: HSteamUser) {
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.release_user)(self.inner.ptr, pipe, user);
        }
    }

    pub fn get_iclient_user_stats(
        &self,
        user: HSteamUser,
        pipe: HSteamPipe,
    ) -> Result<ClientUserStatsMap, SteamClientError> {
        unsafe {
            let vt = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let ptr: *mut IClientUserStatsMap =
                (vt.get_iclient_user_stats)(self.inner.ptr, user, pipe, std::ptr::null());
            if ptr.is_null() {
                Err(SteamClientError::InterfaceCreationFailed(
                    "IClientUserStatsMap".to_owned(),
                ))
            } else {
                Ok(ClientUserStatsMap::from_raw(ptr, self.inner.clone()))
            }
        }
    }

    pub fn get_iclient_user(
        &self,
        user: HSteamUser,
        pipe: HSteamPipe,
    ) -> Result<ClientUser, SteamClientError> {
        unsafe {
            let vt = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamClientError::NullVtable)?;
            let ptr: *mut IClientUser = (vt.get_iclient_user)(self.inner.ptr, user, pipe);
            if ptr.is_null() {
                Err(SteamClientError::InterfaceCreationFailed(
                    "IClientUser".to_owned(),
                ))
            } else {
                Ok(ClientUser::from_raw(ptr, self.inner.clone()))
            }
        }
    }
}
