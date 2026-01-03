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
#![allow(dead_code)]
use crate::steam_client::steamworks_types::{
    AppId_t, CGameID, CSteamID, HSteamUser, SteamAPICall_t, SteamNetworkingIdentity,
};
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct ISteamUserVTable {
    pub get_h_steam_user: unsafe extern "C" fn(*mut ISteamUser) -> HSteamUser,
    pub b_logged_on: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    #[cfg(unix)]
    pub get_steam_id: unsafe extern "C" fn(*mut ISteamUser) -> CSteamID,
    #[cfg(windows)]
    pub get_steam_id: unsafe extern "C" fn(*mut ISteamUser, *mut u64) -> c_int,
    pub initiate_game_connection_deprecated: unsafe extern "C" fn(
        *mut ISteamUser,
        *mut c_void,
        c_int,
        CSteamID,
        u32,
        u16,
        bool,
    ) -> c_int,
    pub terminate_game_connection_deprecated: unsafe extern "C" fn(*mut ISteamUser, u32, u16),
    pub track_app_usage_event: unsafe extern "C" fn(*mut ISteamUser, CGameID, c_int, *const c_char),
    pub get_user_data_folder: unsafe extern "C" fn(*mut ISteamUser, *mut c_char, c_int) -> bool,
    pub start_voice_recording: unsafe extern "C" fn(*mut ISteamUser),
    pub stop_voice_recording: unsafe extern "C" fn(*mut ISteamUser),
    pub get_available_voice:
        unsafe extern "C" fn(*mut ISteamUser, *mut u32, *mut u32, u32) -> c_int,
    pub get_voice: unsafe extern "C" fn(
        *mut ISteamUser,
        bool,
        *mut c_void,
        u32,
        *mut u32,
        bool,
        *mut c_void,
        u32,
        *mut u32,
        u32,
    ) -> c_int,
    pub decompress_voice: unsafe extern "C" fn(
        *mut ISteamUser,
        *const c_void,
        u32,
        *mut c_void,
        u32,
        *mut u32,
        u32,
    ) -> c_int,
    pub get_voice_optimal_sample_rate: unsafe extern "C" fn(*mut ISteamUser) -> u32,
    pub get_auth_session_ticket: unsafe extern "C" fn(
        *mut ISteamUser,
        *mut c_void,
        c_int,
        *mut u32,
        *const SteamNetworkingIdentity,
    ) -> u32,
    pub get_auth_ticket_for_web_api: unsafe extern "C" fn(*mut ISteamUser, *const c_char) -> u32,
    pub begin_auth_session:
        unsafe extern "C" fn(*mut ISteamUser, *const c_void, c_int, CSteamID) -> c_int,
    pub end_auth_session: unsafe extern "C" fn(*mut ISteamUser, CSteamID),
    pub cancel_auth_ticket: unsafe extern "C" fn(*mut ISteamUser, u32),
    pub user_has_license_for_app: unsafe extern "C" fn(*mut ISteamUser, CSteamID, AppId_t) -> c_int,
    pub b_is_behind_nat: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    pub advertise_game: unsafe extern "C" fn(*mut ISteamUser, CSteamID, u32, u16),
    pub request_encrypted_app_ticket:
        unsafe extern "C" fn(*mut ISteamUser, *mut c_void, c_int) -> SteamAPICall_t,
    pub get_encrypted_app_ticket:
        unsafe extern "C" fn(*mut ISteamUser, *mut c_void, c_int, *mut u32) -> bool,
    pub get_game_badge_level: unsafe extern "C" fn(*mut ISteamUser, c_int, bool) -> c_int,
    pub get_player_steam_level: unsafe extern "C" fn(*mut ISteamUser) -> c_int,
    pub request_store_auth_url:
        unsafe extern "C" fn(*mut ISteamUser, *const c_char) -> SteamAPICall_t,
    pub b_is_phone_verified: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    pub b_is_two_factor_enabled: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    pub b_is_phone_identifying: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    pub b_is_phone_requiring_verification: unsafe extern "C" fn(*mut ISteamUser) -> bool,
    pub get_market_eligibility: unsafe extern "C" fn(*mut ISteamUser) -> SteamAPICall_t,
    pub get_duration_control: unsafe extern "C" fn(*mut ISteamUser) -> SteamAPICall_t,
    pub b_set_duration_control_online_state: unsafe extern "C" fn(*mut ISteamUser, c_int) -> bool,
}

#[repr(C)]
pub struct ISteamUser {
    pub vtable: *const ISteamUserVTable,
}

pub const STEAMUSER_INTERFACE_VERSION: &str = "SteamUser023\0";
