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

// Per-app dispatcher for IClientUserStats. Every method takes a
// `CGameID const&` (a pointer in System V x86_64), which is how one
// instance serves any app id without the process being registered as
// running it.
//
// For a regular Steam app, `CGameID` is just the u32 app id zero-extended
// (type=App=0, mod_id=0).

use std::os::raw::c_void;

pub type CGameID = u64;

#[repr(C)]
pub struct IClientUserStatsMap {
    pub vtable: *const IClientUserStatsMapVTable,
}

#[repr(C)]
pub struct IClientUserStatsMapVTable {
    pub get_num_stats: unsafe extern "C" fn(*mut IClientUserStatsMap, *const CGameID) -> u32,
    pub _vt1_get_stat_name: *const c_void,
    pub _vt2_get_stat_type: *const c_void,
    pub get_num_achievements: unsafe extern "C" fn(*mut IClientUserStatsMap, *const CGameID) -> u32,
    pub _vt4_get_achievement_name: *const c_void,
    pub request_current_stats:
        unsafe extern "C" fn(*mut IClientUserStatsMap, *const CGameID) -> bool,
    pub _vt6: *const c_void,
    pub _vt7: *const c_void,
    pub _vt8: *const c_void,
    pub _vt9: *const c_void,
    pub _vt10: *const c_void,
    pub _vt11: *const c_void,
    pub _vt12_get_achievement: *const c_void,
    pub _vt13: *const c_void,
    pub _vt14: *const c_void,
    pub _vt15: *const c_void,
    pub _vt16: *const c_void,
    pub _vt17: *const c_void,
    pub _vt18: *const c_void,
    pub _vt19: *const c_void,
    pub _vt20: *const c_void,
    pub _vt21: *const c_void,
    pub _vt22: *const c_void,
    pub _vt23: *const c_void,
    pub _vt24: *const c_void,
    pub _vt25: *const c_void,
    pub _vt26: *const c_void,
    pub _vt27: *const c_void,
    pub _vt28: *const c_void,
    pub _vt29: *const c_void,
    pub _vt30: *const c_void,
    pub _vt31: *const c_void,
    pub _vt32: *const c_void,
    pub _vt33: *const c_void,
    pub _vt34: *const c_void,
    pub _vt35: *const c_void,
    pub _vt36: *const c_void,
    pub _vt37: *const c_void,
    pub _vt38: *const c_void,
    pub _vt39: *const c_void,
    pub get_num_achieved_achievements:
        unsafe extern "C" fn(*mut IClientUserStatsMap, *const CGameID) -> u32,
}
