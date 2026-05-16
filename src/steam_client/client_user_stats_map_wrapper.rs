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
use crate::steam_client::client_user_stats_map_vtable::{CGameID, IClientUserStatsMap};
use crate::steam_client::steamworks_types::AppId_t;
use std::sync::Arc;

pub struct ClientUserStatsMap {
    inner: Arc<ClientUserStatsMapInner>,
}

struct ClientUserStatsMapInner {
    ptr: *mut IClientUserStatsMap,
    engine: Arc<ClientEngineInner>,
}

impl ClientUserStatsMap {
    pub unsafe fn from_raw(ptr: *mut IClientUserStatsMap, engine: Arc<ClientEngineInner>) -> Self {
        Self {
            inner: Arc::new(ClientUserStatsMapInner { ptr, engine }),
        }
    }

    pub fn run_engine_frame(&self) {
        self.inner.engine.run_frame();
    }

    /// Fire-and-forget: tells Steam to load the per-app schema and per-user
    /// data. The actual load is async — poll `is_schema_loaded` to detect
    /// completion (callbacks don't arrive on the engine pipe).
    pub fn request_current_stats(&self, app_id: AppId_t) -> bool {
        let gid: CGameID = app_id as u64;
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.request_current_stats)(self.inner.ptr, &gid)
        }
    }

    pub fn get_num_stats(&self, app_id: AppId_t) -> u32 {
        let gid: CGameID = app_id as u64;
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.get_num_stats)(self.inner.ptr, &gid)
        }
    }

    /// Schema-loaded signal: either some stats or some achievements exist
    /// in Steam's cache. Apps with stats but no achievements would never
    /// trip `get_num_achievements > 0` alone.
    pub fn is_schema_loaded(&self, app_id: AppId_t) -> bool {
        self.get_num_stats(app_id) > 0 || self.get_num_achievements(app_id) > 0
    }

    pub fn get_num_achievements(&self, app_id: AppId_t) -> u32 {
        let gid: CGameID = app_id as u64;
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.get_num_achievements)(self.inner.ptr, &gid)
        }
    }

    pub fn get_num_achieved_achievements(&self, app_id: AppId_t) -> u32 {
        let gid: CGameID = app_id as u64;
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.get_num_achieved_achievements)(self.inner.ptr, &gid)
        }
    }
}
