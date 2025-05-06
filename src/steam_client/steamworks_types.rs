#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::os::raw::{c_char, c_int, c_void};
use crate::steam_client::steam_client_vtable::ISteamClient;

pub type AppId_t = u32;
pub type DepotId_t = u32;
pub type SteamAPICall_t = u64;
pub type SteamAPIWarningMessageHook_t = unsafe extern "C" fn(c_int, *const c_char);
pub type HSteamPipe = c_int;
pub type HSteamUser = c_int;
pub type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *mut c_int) -> *mut ISteamClient;
pub type SteamGetCallbackFn = unsafe extern "C" fn(HSteamPipe, *mut SteamCallbackMessage, *mut c_int) -> bool;
pub type SteamFreeLastCallbackFn = unsafe extern "C" fn(*const HSteamPipe) -> c_void;

#[allow(non_upper_case_globals)]
const k_cchStatNameMax: usize = 128;
pub type SteamLeaderboard_t = u64;
pub type SteamLeaderboardEntries_t = u64;
pub type UGCHandle_t = u64;

// Define SteamIPAddress_t (simplified - actual implementation may vary)
#[repr(C)]
pub struct SteamIPAddress_t {
    // Implementation depends on actual definition
    _unused: [u8; 0],
}

pub type SteamAPI_CheckCallbackRegistered_t = extern "C" fn();

// Enums from isteamuserstats.h
#[repr(C)]
pub enum ELeaderboardDataRequest {
    Global = 0,
    GlobalAroundUser = 1,
    Friends = 2,
    Users = 3,
}

#[repr(C)]
pub enum ELeaderboardSortMethod {
    None = 0,
    Ascending = 1,
    Descending = 2,
}

#[repr(C)]
pub enum ELeaderboardDisplayType {
    None = 0,
    Numeric = 1,
    TimeSeconds = 2,
    TimeMilliSeconds = 3,
}

#[repr(C)]
pub enum ELeaderboardUploadScoreMethod {
    None = 0,
    KeepBest = 1,
    ForceUpdate = 2,
}

#[repr(C)]
pub struct LeaderboardEntry_t {
    pub m_steamIDUser: CSteamID,
    pub m_nGlobalRank: c_int,
    pub m_nScore: c_int,
    pub m_cDetails: c_int,
    pub m_hUGC: UGCHandle_t,
}

#[repr(C)]
pub struct UserStatsReceived_t {
    pub m_nGameID: u64,
    pub m_eResult: c_int, // EResult
    pub m_steamIDUser: CSteamID,
}

#[repr(C)]
pub struct UserStatsStored_t {
    pub m_nGameID: u64,
    pub m_eResult: c_int, // EResult
}

#[repr(C)]
pub struct UserAchievementStored_t {
    pub m_nGameID: u64,
    pub m_bGroupAchievement: bool,
    pub m_rgchAchievementName: [c_char; k_cchStatNameMax],
    pub m_nCurProgress: u32,
    pub m_nMaxProgress: u32,
}

#[repr(C)]
pub struct LeaderboardFindResult_t {
    pub m_hSteamLeaderboard: SteamLeaderboard_t,
    pub m_bLeaderboardFound: u8,
}

#[repr(C)]
pub struct LeaderboardScoresDownloaded_t {
    pub m_hSteamLeaderboard: SteamLeaderboard_t,
    pub m_hSteamLeaderboardEntries: SteamLeaderboardEntries_t,
    pub m_cEntryCount: c_int,
}

#[repr(C)]
pub struct LeaderboardScoreUploaded_t {
    pub m_bSuccess: u8,
    pub m_hSteamLeaderboard: SteamLeaderboard_t,
    pub m_nScore: c_int,
    pub m_bScoreChanged: u8,
    pub m_nGlobalRankNew: c_int,
    pub m_nGlobalRankPrevious: c_int,
}

#[repr(C)]
pub struct NumberOfCurrentPlayers_t {
    pub m_bSuccess: u8,
    pub m_cPlayers: c_int,
}

#[repr(C)]
pub struct UserStatsUnloaded_t {
    pub m_steamIDUser: CSteamID,
}

#[repr(C)]
pub struct UserAchievementIconFetched_t {
    pub m_nGameID: u64,
    pub m_rgchAchievementName: [c_char; k_cchStatNameMax],
    pub m_bAchieved: bool,
    pub m_nIconHandle: c_int,
}

#[repr(C)]
pub struct GlobalAchievementPercentagesReady_t {
    pub m_nGameID: u64,
    pub m_eResult: c_int, // EResult
}

#[repr(C)]
pub struct LeaderboardUGCSet_t {
    pub m_eResult: c_int, // EResult
    pub m_hSteamLeaderboard: SteamLeaderboard_t,
}

#[repr(C)]
pub struct GlobalStatsReceived_t {
    pub m_nGameID: u64,
    pub m_eResult: c_int, // EResult
}

#[repr(C)]
pub struct SteamCallbackMessage {
    pub user: HSteamPipe,
    pub id: c_int,
    pub param_ptr: *mut c_int,
    pub param_size: c_int,
}

#[repr(C)]
pub struct CSteamID {
    pub m_steamid: u64,
}

#[repr(C)]
pub enum EAccountType {
    k_EAccountTypeInvalid = 0,
    k_EAccountTypeIndividual = 1,
    // ... other account types
}


#[repr(C)]
pub enum ESteamAPICallFailure {
    None = -1,
    SteamGone = 0,
    NetworkFailure = 1,
    InvalidHandle = 2,
    MismatchedCallback = 3,
}

#[repr(C)]
pub enum EGamepadTextInputMode {
    Normal = 0,
    Password = 1,
}

#[repr(C)]
pub enum EGamepadTextInputLineMode {
    SingleLine = 0,
    MultipleLines = 1,
}

#[repr(C)]
pub enum EFloatingGamepadTextInputMode {
    ModeSingleLine = 0,
    ModeMultipleLines = 1,
    ModeEmail = 2,
    ModeNumeric = 3,
}

#[repr(C)]
pub enum ETextFilteringContext {
    Unknown = 0,
    GameContent = 1,
    Chat = 2,
    Name = 3,
}

#[repr(C)]
pub enum EUniverse {
    Invalid = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    Max = 5,
}

#[repr(C)]
pub enum ENotificationPosition {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

#[repr(C)]
pub enum ESteamIPv6ConnectivityState {
    Unknown = 0,
    Good = 1,
    Bad = 2,
}

#[repr(C)]
pub enum ESteamIPv6ConnectivityProtocol {
    HTTP = 0,
    UDP = 1,
}

#[repr(C)]
pub enum ECheckFileSignature {
    InvalidSignature = 0,
    ValidSignature = 1,
    FileNotFound = 2,
    NoSignaturesFoundForThisApp = 3,
    NoSignaturesFoundForThisFile = 4,
}
