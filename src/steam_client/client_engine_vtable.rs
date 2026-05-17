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

use std::os::raw::{c_char, c_void};

use crate::steam_client::client_user_stats_map_vtable::IClientUserStatsMap;
use crate::steam_client::client_user_vtable::IClientUser;
use crate::steam_client::steamworks_types::{HSteamPipe, HSteamUser};

#[repr(C)]
pub struct IClientEngine {
    pub vtable: *const IClientEngineVTable,
}

#[repr(C)]
pub struct IClientEngineVTable {
    pub create_steam_pipe: unsafe extern "C" fn(*mut IClientEngine) -> HSteamPipe,
    pub release_steam_pipe: unsafe extern "C" fn(*mut IClientEngine, HSteamPipe) -> bool,
    pub _vt2_create_global_user: *const c_void,
    pub connect_to_global_user: unsafe extern "C" fn(*mut IClientEngine, HSteamPipe) -> HSteamUser,
    pub _vt4: *const c_void,
    pub _vt5: *const c_void,
    pub release_user: unsafe extern "C" fn(*mut IClientEngine, HSteamPipe, HSteamUser),
    pub _vt7_is_valid_pipe: *const c_void,
    pub get_iclient_user:
        unsafe extern "C" fn(*mut IClientEngine, HSteamUser, HSteamPipe) -> *mut IClientUser,
    pub _vt9_get_iclient_game_server: *const c_void,
    pub _vt10: *const c_void,
    pub _vt11_set_local_ip_binding: *const c_void,
    pub _vt12_get_universe_name: *const c_void,
    pub _vt13_get_iclient_friends: *const c_void,
    pub _vt14_get_iclient_utils: *const c_void,
    pub _vt15_get_iclient_billing: *const c_void,
    pub _vt16_get_iclient_matchmaking: *const c_void,
    pub _vt17_get_iclient_apps: *const c_void,
    pub _vt18_get_iclient_matchmaking_servers: *const c_void,
    pub run_frame: unsafe extern "C" fn(*mut IClientEngine),
    pub _vt20_get_ipc_call_count: *const c_void,
    pub get_iclient_user_stats: unsafe extern "C" fn(
        *mut IClientEngine,
        HSteamUser,
        HSteamPipe,
        *const c_char,
    ) -> *mut IClientUserStatsMap,
}

pub const CLIENTENGINE_INTERFACE_VERSION: &str = "CLIENTENGINE_INTERFACE_VERSION005\0";
