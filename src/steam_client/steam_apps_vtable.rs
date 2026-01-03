#![allow(dead_code)]
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

use crate::steam_client::steamworks_types::{AppId_t, CSteamID, DepotId_t, SteamAPICall_t};
use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct ISteamAppsVTable {
    pub b_is_subscribed: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub b_is_low_violence: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub b_is_cybercafe: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub b_is_vac_banned: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub get_current_game_language: unsafe extern "C" fn(*mut ISteamApps) -> *const c_char,
    pub get_available_game_languages: unsafe extern "C" fn(*mut ISteamApps) -> *const c_char,
    pub b_is_subscribed_app: unsafe extern "C" fn(*mut ISteamApps, AppId_t) -> bool,
    pub b_is_dlc_installed: unsafe extern "C" fn(*mut ISteamApps, AppId_t) -> bool,
    pub get_earliest_purchase_unix_time: unsafe extern "C" fn(*mut ISteamApps, AppId_t) -> u32,
    pub b_is_subscribed_from_free_weekend: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub get_dlc_count: unsafe extern "C" fn(*mut ISteamApps) -> c_int,
    pub b_get_dlc_data_by_index: unsafe extern "C" fn(
        *mut ISteamApps,
        c_int,
        *mut AppId_t,
        *mut bool,
        *mut c_char,
        c_int,
    ) -> bool,
    pub install_dlc: unsafe extern "C" fn(*mut ISteamApps, AppId_t),
    pub uninstall_dlc: unsafe extern "C" fn(*mut ISteamApps, AppId_t),
    pub request_app_proof_of_purchase_key: unsafe extern "C" fn(*mut ISteamApps, AppId_t),
    pub get_current_beta_name: unsafe extern "C" fn(*mut ISteamApps, *mut c_char, c_int) -> bool,
    pub mark_content_corrupt: unsafe extern "C" fn(*mut ISteamApps, bool) -> bool,
    pub get_installed_depots:
        unsafe extern "C" fn(*mut ISteamApps, AppId_t, *mut DepotId_t, u32) -> u32,
    pub get_app_install_dir:
        unsafe extern "C" fn(*mut ISteamApps, AppId_t, *mut c_char, u32) -> u32,
    pub b_is_app_installed: unsafe extern "C" fn(*mut ISteamApps, AppId_t) -> bool,
    pub get_app_owner: unsafe extern "C" fn(*mut ISteamApps) -> CSteamID,
    pub get_launch_query_param:
        unsafe extern "C" fn(*mut ISteamApps, *const c_char) -> *const c_char,
    pub get_dlc_download_progress:
        unsafe extern "C" fn(*mut ISteamApps, AppId_t, *mut u64, *mut u64) -> bool,
    pub get_app_build_id: unsafe extern "C" fn(*mut ISteamApps) -> c_int,
    pub request_all_proof_of_purchase_keys: unsafe extern "C" fn(*mut ISteamApps),
    pub get_file_details: unsafe extern "C" fn(*mut ISteamApps, *const c_char) -> SteamAPICall_t,
    pub get_launch_command_line: unsafe extern "C" fn(*mut ISteamApps, *mut c_char, c_int) -> c_int,
    pub b_is_subscribed_from_family_sharing: unsafe extern "C" fn(*mut ISteamApps) -> bool,
    pub b_is_timed_trial: unsafe extern "C" fn(*mut ISteamApps, *mut u32, *mut u32) -> bool,
    pub set_dlc_context: unsafe extern "C" fn(*mut ISteamApps, AppId_t) -> bool,
}

#[repr(C)]
pub struct ISteamApps {
    pub vtable: *const ISteamAppsVTable,
}

pub const STEAMAPPS_INTERFACE_VERSION: &str = "STEAMAPPS_INTERFACE_VERSION008\0";
