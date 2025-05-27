#![allow(dead_code)]

use std::os::raw::{c_int, c_char, c_float};
use crate::steam_client::steamworks_types::{SteamAPICall_t, CSteamID, ELeaderboardSortMethod, SteamLeaderboard_t, ELeaderboardDisplayType, ELeaderboardDataRequest, SteamLeaderboardEntries_t, ELeaderboardUploadScoreMethod, UGCHandle_t, LeaderboardEntry_t};

#[repr(C)]
pub struct ISteamUserStatsVTable {
    #[cfg(target_os = "windows")]
    pub get_stat_float: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_float) -> bool,
    #[cfg(target_os = "windows")]
    pub get_stat_int32: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_int) -> bool,
    #[cfg(target_os = "windows")]
    pub set_stat_float: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, c_float) -> bool,
    #[cfg(target_os = "windows")]
    pub set_stat_int32: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, c_int) -> bool,

    #[cfg(not(target_os = "windows"))]
    pub get_stat_int32: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_int) -> bool,
    #[cfg(not(target_os = "windows"))]
    pub get_stat_float: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_float) -> bool,
    #[cfg(not(target_os = "windows"))]
    pub set_stat_int32: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, c_int) -> bool,
    #[cfg(not(target_os = "windows"))]
    pub set_stat_float: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, c_float) -> bool,

    pub update_avg_rate_stat: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, c_float, f64) -> bool,
    pub get_achievement: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut bool) -> bool,
    pub set_achievement: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char) -> bool,
    pub clear_achievement: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char) -> bool,
    pub get_achievement_and_unlock_time: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut bool, *mut u32) -> bool,
    pub store_stats: unsafe extern "C" fn(*mut ISteamUserStats) -> bool,
    pub get_achievement_icon: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char) -> c_int,
    pub get_achievement_display_attribute: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *const c_char) -> *const c_char,
    pub indicate_achievement_progress: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, u32, u32) -> bool,
    pub get_num_achievements: unsafe extern "C" fn(*mut ISteamUserStats) -> u32,
    pub get_achievement_name: unsafe extern "C" fn(*mut ISteamUserStats, u32) -> *const c_char,
    pub request_user_stats: unsafe extern "C" fn(*mut ISteamUserStats, CSteamID) -> SteamAPICall_t,
    pub get_user_stat_int32: unsafe extern "C" fn(*mut ISteamUserStats, CSteamID, *const c_char, *mut c_int) -> bool,
    pub get_user_stat_float: unsafe extern "C" fn(*mut ISteamUserStats, CSteamID, *const c_char, *mut c_float) -> bool,
    pub get_user_achievement: unsafe extern "C" fn(*mut ISteamUserStats, CSteamID, *const c_char, *mut bool) -> bool,
    pub get_user_achievement_and_unlock_time: unsafe extern "C" fn(*mut ISteamUserStats, CSteamID, *const c_char, *mut bool, *mut u32) -> bool,
    pub reset_all_stats: unsafe extern "C" fn(*mut ISteamUserStats, bool) -> bool,
    pub find_or_create_leaderboard: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, ELeaderboardSortMethod, ELeaderboardDisplayType) -> SteamAPICall_t,
    pub find_leaderboard: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char) -> SteamAPICall_t,
    pub get_leaderboard_name: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t) -> *const c_char,
    pub get_leaderboard_entry_count: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t) -> c_int,
    pub get_leaderboard_sort_method: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t) -> ELeaderboardSortMethod,
    pub get_leaderboard_display_type: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t) -> ELeaderboardDisplayType,
    pub download_leaderboard_entries: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t, ELeaderboardDataRequest, c_int, c_int) -> SteamAPICall_t,
    pub download_leaderboard_entries_for_users: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t, *mut CSteamID, c_int) -> SteamAPICall_t,
    pub get_downloaded_leaderboard_entry: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboardEntries_t, c_int, *mut LeaderboardEntry_t, *mut c_int, c_int) -> bool,
    pub upload_leaderboard_score: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t, ELeaderboardUploadScoreMethod, c_int, *const c_int, c_int) -> SteamAPICall_t,
    pub attach_leaderboard_ugc: unsafe extern "C" fn(*mut ISteamUserStats, SteamLeaderboard_t, UGCHandle_t) -> SteamAPICall_t,
    pub get_number_of_current_players: unsafe extern "C" fn(*mut ISteamUserStats) -> SteamAPICall_t,
    pub request_global_achievement_percentages: unsafe extern "C" fn(*mut ISteamUserStats) -> SteamAPICall_t,
    pub get_most_achieved_achievement_info: unsafe extern "C" fn(*mut ISteamUserStats, *mut c_char, u32, *mut c_float, *mut bool) -> c_int,
    pub get_next_most_achieved_achievement_info: unsafe extern "C" fn(*mut ISteamUserStats, c_int, *mut c_char, u32, *mut c_float, *mut bool) -> c_int,
    pub get_achievement_achieved_percent: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_float) -> bool,
    pub request_global_stats: unsafe extern "C" fn(*mut ISteamUserStats, c_int) -> SteamAPICall_t,
    pub get_global_stat_int64: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut i64) -> bool,
    pub get_global_stat_double: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut f64) -> bool,
    pub get_global_stat_history_int64: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut i64, u32) -> c_int,
    pub get_global_stat_history_double: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut f64, u32) -> c_int,
    pub get_achievement_progress_limits_int32: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_int, *mut c_int) -> bool,
    pub get_achievement_progress_limits_float: unsafe extern "C" fn(*mut ISteamUserStats, *const c_char, *mut c_float, *mut c_float) -> bool,
}

#[repr(C)]
pub struct ISteamUserStats {
    pub vtable: *const ISteamUserStatsVTable,
}

pub const STEAMUSERSTATS_INTERFACE_VERSION: &str = "STEAMUSERSTATS_INTERFACE_VERSION013\0";