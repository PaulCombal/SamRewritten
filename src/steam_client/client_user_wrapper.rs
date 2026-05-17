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

use crate::steam_client::client_engine_wrapper::ClientEngineInner;
use crate::steam_client::client_user_vtable::IClientUser;
use crate::steam_client::steamworks_types::AppId_t;
use std::sync::Arc;

pub struct ClientUser {
    inner: Arc<ClientUserInner>,
}

struct ClientUserInner {
    ptr: *mut IClientUser,
    _engine: Arc<ClientEngineInner>,
}

impl ClientUser {
    pub unsafe fn from_raw(ptr: *mut IClientUser, engine: Arc<ClientEngineInner>) -> Self {
        Self {
            inner: Arc::new(ClientUserInner {
                ptr,
                _engine: engine,
            }),
        }
    }

    /// Returns the full list of app ids the user is subscribed to (owns,
    /// rents, has via family sharing, …). One IPC roundtrip for the count,
    /// one for the data — irrespective of library size.
    pub fn get_subscribed_apps(&self) -> Vec<AppId_t> {
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            let count = (vt.get_subscribed_apps)(self.inner.ptr, std::ptr::null_mut(), 0, false);
            let mut buf: Vec<AppId_t> = vec![0; count as usize];
            let written = (vt.get_subscribed_apps)(self.inner.ptr, buf.as_mut_ptr(), count, false);
            buf.truncate(written as usize);
            buf
        }
    }
}
