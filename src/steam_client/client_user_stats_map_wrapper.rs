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
use crate::steam_client::create_client::callback_pump;
use crate::steam_client::steamworks_types::{
    AppId_t, HSteamPipe, SteamCallbackMessage, UserStatsReceived_t,
};
use crate::steam_client::wrapper_types::SteamCallbackId;
use std::mem::offset_of;
use std::os::raw::c_int;
use std::rc::Rc;

pub struct ClientUserStatsMap {
    inner: Rc<ClientUserStatsMapInner>,
}

struct ClientUserStatsMapInner {
    ptr: *mut IClientUserStatsMap,
    engine: Rc<ClientEngineInner>,
    pipe: HSteamPipe,
}

impl ClientUserStatsMap {
    pub unsafe fn from_raw(
        ptr: *mut IClientUserStatsMap,
        engine: Rc<ClientEngineInner>,
        pipe: HSteamPipe,
    ) -> Self {
        Self {
            inner: Rc::new(ClientUserStatsMapInner { ptr, engine, pipe }),
        }
    }

    /// Result kept raw: an `EResult` Valve added since is not a valid enum.
    pub fn drain_user_stats_callbacks(&self) -> Vec<(AppId_t, i32)> {
        let Some((get, free)) = callback_pump() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        loop {
            let mut msg = SteamCallbackMessage {
                user: 0,
                id: 0,
                param_ptr: std::ptr::null_mut(),
                param_size: 0,
            };
            let mut call: c_int = 0;
            unsafe {
                if !get(self.inner.pipe, &mut msg, &mut call) {
                    break;
                }
                if msg.id == SteamCallbackId::UserStatsReceived as c_int
                    && !msg.param_ptr.is_null()
                    && msg.param_size >= size_of::<UserStatsReceived_t>() as c_int
                {
                    let base = msg.param_ptr as *const u8;
                    let game = base.add(offset_of!(UserStatsReceived_t, m_nGameID));
                    let result = base.add(offset_of!(UserStatsReceived_t, m_eResult));
                    out.push((
                        std::ptr::read_unaligned(game as *const u64) as AppId_t,
                        std::ptr::read_unaligned(result as *const i32),
                    ));
                }
                free(self.inner.pipe);
            }
        }
        out
    }

    pub fn run_engine_frame(&self) {
        self.inner.engine.run_frame();
    }

    pub fn request_current_stats(&self, app_id: AppId_t) -> bool {
        let gid: CGameID = app_id as u64;
        unsafe {
            let vt = (*self.inner.ptr).vtable.as_ref().expect("vtable null");
            (vt.request_current_stats)(self.inner.ptr, &gid)
        }
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
