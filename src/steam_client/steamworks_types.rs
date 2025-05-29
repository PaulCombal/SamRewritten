#![allow(non_camel_case_types, non_snake_case, dead_code)]
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


use std::os::raw::{c_char, c_int};
use crate::steam_client::steam_client_vtable::ISteamClient;

pub type AppId_t = u32;
pub type DepotId_t = u32;
pub type SteamAPICall_t = u64; // 0 -> Invalid
pub type SteamAPIWarningMessageHook_t = unsafe extern "C" fn(c_int, *const c_char);
pub type HSteamPipe = c_int;
pub type HSteamUser = c_int;
pub type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *mut c_int) -> *mut ISteamClient;
pub type SteamGetCallbackFn = unsafe extern "C" fn(HSteamPipe, *mut SteamCallbackMessage, *mut c_int) -> bool;
pub type SteamFreeLastCallbackFn = unsafe extern "C" fn(HSteamPipe) -> bool;

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

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EResult {
    k_EResultNone = 0,
    k_EResultOK = 1,
    k_EResultFail = 2,
    k_EResultNoConnection = 3,
    // k_EResultNoConnectionRetry = 4,  // OBSOLETE - removed
    k_EResultInvalidPassword = 5,
    k_EResultLoggedInElsewhere = 6,
    k_EResultInvalidProtocolVer = 7,
    k_EResultInvalidParam = 8,
    k_EResultFileNotFound = 9,
    k_EResultBusy = 10,
    k_EResultInvalidState = 11,
    k_EResultInvalidName = 12,
    k_EResultInvalidEmail = 13,
    k_EResultDuplicateName = 14,
    k_EResultAccessDenied = 15,
    k_EResultTimeout = 16,
    k_EResultBanned = 17,
    k_EResultAccountNotFound = 18,
    k_EResultInvalidSteamID = 19,
    k_EResultServiceUnavailable = 20,
    k_EResultNotLoggedOn = 21,
    k_EResultPending = 22,
    k_EResultEncryptionFailure = 23,
    k_EResultInsufficientPrivilege = 24,
    k_EResultLimitExceeded = 25,
    k_EResultRevoked = 26,
    k_EResultExpired = 27,
    k_EResultAlreadyRedeemed = 28,
    k_EResultDuplicateRequest = 29,
    k_EResultAlreadyOwned = 30,
    k_EResultIPNotFound = 31,
    k_EResultPersistFailed = 32,
    k_EResultLockingFailed = 33,
    k_EResultLogonSessionReplaced = 34,
    k_EResultConnectFailed = 35,
    k_EResultHandshakeFailed = 36,
    k_EResultIOFailure = 37,
    k_EResultRemoteDisconnect = 38,
    k_EResultShoppingCartNotFound = 39,
    k_EResultBlocked = 40,
    k_EResultIgnored = 41,
    k_EResultNoMatch = 42,
    k_EResultAccountDisabled = 43,
    k_EResultServiceReadOnly = 44,
    k_EResultAccountNotFeatured = 45,
    k_EResultAdministratorOK = 46,
    k_EResultContentVersion = 47,
    k_EResultTryAnotherCM = 48,
    k_EResultPasswordRequiredToKickSession = 49,
    k_EResultAlreadyLoggedInElsewhere = 50,
    k_EResultSuspended = 51,
    k_EResultCancelled = 52,
    k_EResultDataCorruption = 53,
    k_EResultDiskFull = 54,
    k_EResultRemoteCallFailed = 55,
    k_EResultPasswordUnset = 56,
    k_EResultExternalAccountUnlinked = 57,
    k_EResultPSNTicketInvalid = 58,
    k_EResultExternalAccountAlreadyLinked = 59,
    k_EResultRemoteFileConflict = 60,
    k_EResultIllegalPassword = 61,
    k_EResultSameAsPreviousValue = 62,
    k_EResultAccountLogonDenied = 63,
    k_EResultCannotUseOldPassword = 64,
    k_EResultInvalidLoginAuthCode = 65,
    k_EResultAccountLogonDeniedNoMail = 66,
    k_EResultHardwareNotCapableOfIPT = 67,
    k_EResultIPTInitError = 68,
    k_EResultParentalControlRestricted = 69,
    k_EResultFacebookQueryError = 70,
    k_EResultExpiredLoginAuthCode = 71,
    k_EResultIPLoginRestrictionFailed = 72,
    k_EResultAccountLockedDown = 73,
    k_EResultAccountLogonDeniedVerifiedEmailRequired = 74,
    k_EResultNoMatchingURL = 75,
    k_EResultBadResponse = 76,
    k_EResultRequirePasswordReEntry = 77,
    k_EResultValueOutOfRange = 78,
    k_EResultUnexpectedError = 79,
    k_EResultDisabled = 80,
    k_EResultInvalidCEGSubmission = 81,
    k_EResultRestrictedDevice = 82,
    k_EResultRegionLocked = 83,
    k_EResultRateLimitExceeded = 84,
    k_EResultAccountLoginDeniedNeedTwoFactor = 85,
    k_EResultItemDeleted = 86,
    k_EResultAccountLoginDeniedThrottle = 87,
    k_EResultTwoFactorCodeMismatch = 88,
    k_EResultTwoFactorActivationCodeMismatch = 89,
    k_EResultAccountAssociatedToMultiplePartners = 90,
    k_EResultNotModified = 91,
    k_EResultNoMobileDevice = 92,
    k_EResultTimeNotSynced = 93,
    k_EResultSmsCodeFailed = 94,
    k_EResultAccountLimitExceeded = 95,
    k_EResultAccountActivityLimitExceeded = 96,
    k_EResultPhoneActivityLimitExceeded = 97,
    k_EResultRefundToWallet = 98,
    k_EResultEmailSendFailure = 99,
    k_EResultNotSettled = 100,
    k_EResultNeedCaptcha = 101,
    k_EResultGSLTDenied = 102,
    k_EResultGSOwnerDenied = 103,
    k_EResultInvalidItemType = 104,
    k_EResultIPBanned = 105,
    k_EResultGSLTExpired = 106,
    k_EResultInsufficientFunds = 107,
    k_EResultTooManyPending = 108,
    k_EResultNoSiteLicensesFound = 109,
    k_EResultWGNetworkSendExceeded = 110,
    k_EResultAccountNotFriends = 111,
    k_EResultLimitedUserAccount = 112,
    k_EResultCantRemoveItem = 113,
    k_EResultAccountDeleted = 114,
    k_EResultExistingUserCancelledLicense = 115,
    k_EResultCommunityCooldown = 116,
    k_EResultNoLauncherSpecified = 117,
    k_EResultMustAgreeToSSA = 118,
    k_EResultLauncherMigrated = 119,
    k_EResultSteamRealmMismatch = 120,
    k_EResultInvalidSignature = 121,
    k_EResultParseFailure = 122,
    k_EResultNoVerifiedPhone = 123,
    k_EResultInsufficientBattery = 124,
    k_EResultChargerRequired = 125,
    k_EResultCachedCredentialInvalid = 126,
    K_EResultPhoneNumberIsVOIP = 127,
    k_EResultNotSupported = 128,
    k_EResultFamilySizeLimitExceeded = 129,
    k_EResultOfflineAppCacheInvalid = 130,
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
    pub m_eResult: EResult,
    pub m_steamIDUser: CSteamID,
}

#[repr(C)]
pub struct UserStatsStored_t {
    pub m_nGameID: u64,
    pub m_eResult: EResult,
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
#[derive(Debug)]
pub struct GlobalAchievementPercentagesReady_t {
    pub m_nGameID: u64,
    pub m_eResult: EResult
}

#[repr(C)]
pub struct LeaderboardUGCSet_t {
    pub m_eResult: EResult,
    pub m_hSteamLeaderboard: SteamLeaderboard_t,
}

#[repr(C)]
pub struct GlobalStatsReceived_t {
    pub m_nGameID: u64,
    pub m_eResult: EResult
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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
