#![allow(dead_code)]
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

use crate::steam_client::steamworks_types::AppId_t;
use std::os::raw::{c_char, c_int};

// You need to be whitelisted by Valve to use this interface.
// This is simply in the codebase for reference.
// For tinkerers around, I've read that Spacewar is a whitelisted app.

#[repr(C)]
pub struct ISteamAppListVTable {
    pub get_num_installed_apps: unsafe extern "C" fn(*mut ISteamAppList) -> u32,
    pub get_installed_apps: unsafe extern "C" fn(*mut ISteamAppList, *mut AppId_t, u32) -> u32,
    pub get_app_name:
        unsafe extern "C" fn(*mut ISteamAppList, AppId_t, *mut c_char, c_int) -> c_int,
    pub get_app_install_dir:
        unsafe extern "C" fn(*mut ISteamAppList, AppId_t, *mut c_char, c_int) -> c_int,
    pub get_app_build_id: unsafe extern "C" fn(*mut ISteamAppList, AppId_t) -> c_int,
}

#[repr(C)]
pub struct ISteamAppList {
    pub vtable: *const ISteamAppListVTable,
}

pub const STEAMAPPLIST_INTERFACE_VERSION: &str = "STEAMAPPLIST_INTERFACE_VERSION001\0";
