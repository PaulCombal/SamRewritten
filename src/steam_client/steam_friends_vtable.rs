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
    AppId_t, CSteamID, EActivateGameOverlayToWebPageMode, EChatEntryType,
    ECommunityProfileItemProperty, ECommunityProfileItemType, EFriendRelationship, EOverlayToStoreFlag,
    EPersonaState, FriendGameInfo_t, FriendsGroupID_t, SteamAPICall_t,
};
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct ISteamFriendsVTable {
    pub get_persona_name: unsafe extern "C" fn(*mut ISteamFriends) -> *const c_char,
    pub get_persona_state: unsafe extern "C" fn(*mut ISteamFriends) -> EPersonaState,
    pub get_friend_count: unsafe extern "C" fn(*mut ISteamFriends, c_int) -> c_int,
    pub get_friend_by_index: unsafe extern "C" fn(*mut ISteamFriends, c_int, c_int) -> CSteamID,
    pub get_friend_relationship:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> EFriendRelationship,
    pub get_friend_persona_state: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> EPersonaState,
    pub get_friend_persona_name: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> *const c_char,
    pub get_friend_game_played:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *mut FriendGameInfo_t) -> bool,
    pub get_friend_persona_name_history:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> *const c_char,
    pub get_friend_steam_level: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_player_nickname: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> *const c_char,
    pub get_friends_group_count: unsafe extern "C" fn(*mut ISteamFriends) -> c_int,
    pub get_friends_group_id_by_index:
        unsafe extern "C" fn(*mut ISteamFriends, c_int) -> FriendsGroupID_t,
    pub get_friends_group_name:
        unsafe extern "C" fn(*mut ISteamFriends, FriendsGroupID_t) -> *const c_char,
    pub get_friends_group_members_count:
        unsafe extern "C" fn(*mut ISteamFriends, FriendsGroupID_t) -> c_int,
    pub get_friends_group_members_list:
        unsafe extern "C" fn(*mut ISteamFriends, FriendsGroupID_t, *mut CSteamID, c_int),
    pub has_friend: unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> bool,
    pub get_clan_count: unsafe extern "C" fn(*mut ISteamFriends) -> c_int,
    pub get_clan_by_index: unsafe extern "C" fn(*mut ISteamFriends, c_int) -> CSteamID,
    pub get_clan_name: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> *const c_char,
    pub get_clan_tag: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> *const c_char,
    pub get_clan_activity_counts:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *mut c_int, *mut c_int, *mut c_int) -> bool,
    pub download_clan_activity_counts:
        unsafe extern "C" fn(*mut ISteamFriends, *mut CSteamID, c_int) -> SteamAPICall_t,
    pub get_friend_count_from_source: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_friend_from_source_by_index:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> CSteamID,
    pub is_user_in_source: unsafe extern "C" fn(*mut ISteamFriends, CSteamID, CSteamID) -> bool,
    pub set_in_game_voice_speaking: unsafe extern "C" fn(*mut ISteamFriends, CSteamID, bool),
    pub activate_game_overlay: unsafe extern "C" fn(*mut ISteamFriends, *const c_char),
    pub activate_game_overlay_to_user:
        unsafe extern "C" fn(*mut ISteamFriends, *const c_char, CSteamID),
    pub activate_game_overlay_to_web_page:
        unsafe extern "C" fn(*mut ISteamFriends, *const c_char, EActivateGameOverlayToWebPageMode),
    pub activate_game_overlay_to_store:
        unsafe extern "C" fn(*mut ISteamFriends, AppId_t, EOverlayToStoreFlag),
    pub set_played_with: unsafe extern "C" fn(*mut ISteamFriends, CSteamID),
    pub activate_game_overlay_invite_dialog: unsafe extern "C" fn(*mut ISteamFriends, CSteamID),
    pub get_small_friend_avatar: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_medium_friend_avatar: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_large_friend_avatar: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub request_user_information: unsafe extern "C" fn(*mut ISteamFriends, CSteamID, bool) -> bool,
    pub request_clan_officer_list: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> SteamAPICall_t,
    pub get_clan_owner: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> CSteamID,
    pub get_clan_officer_count: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_clan_officer_by_index:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> CSteamID,
    pub set_rich_presence:
        unsafe extern "C" fn(*mut ISteamFriends, *const c_char, *const c_char) -> bool,
    pub clear_rich_presence: unsafe extern "C" fn(*mut ISteamFriends),
    pub get_friend_rich_presence:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *const c_char) -> *const c_char,
    pub get_friend_rich_presence_key_count: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_friend_rich_presence_key_by_index:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> *const c_char,
    pub request_friend_rich_presence: unsafe extern "C" fn(*mut ISteamFriends, CSteamID),
    pub invite_user_to_game:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *const c_char) -> bool,
    pub get_coplay_friend_count: unsafe extern "C" fn(*mut ISteamFriends) -> c_int,
    pub get_coplay_friend: unsafe extern "C" fn(*mut ISteamFriends, c_int) -> CSteamID,
    pub get_friend_coplay_time: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_friend_coplay_game: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> AppId_t,
    pub join_clan_chat_room: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> SteamAPICall_t,
    pub leave_clan_chat_room: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub get_clan_chat_member_count: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> c_int,
    pub get_chat_member_by_index:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, c_int) -> CSteamID,
    pub send_clan_chat_message:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *const c_char) -> bool,
    pub get_clan_chat_message: unsafe extern "C" fn(
        *mut ISteamFriends,
        CSteamID,
        c_int,
        *mut c_void,
        c_int,
        *mut EChatEntryType,
        *mut CSteamID,
    ) -> c_int,
    pub is_clan_chat_admin: unsafe extern "C" fn(*mut ISteamFriends, CSteamID, CSteamID) -> bool,
    pub is_clan_chat_window_open_in_steam: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub open_clan_chat_window_in_steam: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub close_clan_chat_window_in_steam: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub set_listen_for_friends_messages: unsafe extern "C" fn(*mut ISteamFriends, bool) -> bool,
    pub reply_to_friend_message:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, *const c_char) -> bool,
    pub get_friend_message: unsafe extern "C" fn(
        *mut ISteamFriends,
        CSteamID,
        c_int,
        *mut c_void,
        c_int,
        *mut EChatEntryType,
    ) -> c_int,
    pub get_follower_count: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> SteamAPICall_t,
    pub is_following: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> SteamAPICall_t,
    pub enumerate_following_list: unsafe extern "C" fn(*mut ISteamFriends, u32) -> SteamAPICall_t,
    pub is_clan_public: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub is_clan_official_game_group: unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> bool,
    pub get_num_chats_with_unread_priority_messages: unsafe extern "C" fn(*mut ISteamFriends) -> c_int,
    pub activate_game_overlay_remote_play_together_invite_dialog:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID),
    pub register_protocol_in_overlay_browser:
        unsafe extern "C" fn(*mut ISteamFriends, *const c_char) -> bool,
    pub activate_game_overlay_invite_dialog_connect_string:
        unsafe extern "C" fn(*mut ISteamFriends, *const c_char),
    pub request_equipped_profile_items:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID) -> SteamAPICall_t,
    pub bhas_equipped_profile_item:
        unsafe extern "C" fn(*mut ISteamFriends, CSteamID, ECommunityProfileItemType) -> bool,
    pub get_profile_item_property_string: unsafe extern "C" fn(
        *mut ISteamFriends,
        CSteamID,
        ECommunityProfileItemType,
        ECommunityProfileItemProperty,
    ) -> *const c_char,
    pub get_profile_item_property_uint: unsafe extern "C" fn(
        *mut ISteamFriends,
        CSteamID,
        ECommunityProfileItemType,
        ECommunityProfileItemProperty,
    ) -> u32,
}

#[repr(C)]
pub struct ISteamFriends {
    pub vtable: *const ISteamFriendsVTable,
}

pub const STEAMFRIENDS_INTERFACE_VERSION: &str = "SteamFriends018\0";
