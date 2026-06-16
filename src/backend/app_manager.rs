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

use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::key_value::{KeyValue, KeyValueData};
use crate::backend::stat_definitions::{
    AchievementDefinition, AchievementInfo, BaseStatDefinition, FloatStatDefinition, FloatStatInfo,
    IntStatInfo, IntegerStatDefinition, StatDefinition, StatInfo,
};
use crate::backend::types::UserStatType;
use crate::backend::user_unlock_times::{self, AchievementUnlock};
use crate::dev_println;
use crate::steam_client::steamworks_types::{
    AppId_t, CSteamID, EResult, GlobalAchievementPercentagesReady_t, UserStatsReceived_t,
};
use crate::steam_client::wrapper_types::SteamCallbackId;
use crate::utils::ipc_types::SamError;
use crate::utils::steam_locator::SteamLocator;
use std::env;
use std::time::UNIX_EPOCH;

pub struct AppManager {
    app_id: AppId_t,
    connected_steam: ConnectedSteam,
    definitions_loaded: bool,
    user_stats_received: bool,
    achievement_definitions: Vec<AchievementDefinition>,
    stat_definitions: Vec<StatDefinition>,
}

pub struct StatState<T> {
    pub min: T,
    pub max: T,
    pub increment_only: bool,
    pub default: T,
    pub current: Option<T>,
}

#[cfg(any(debug_assertions, test))]
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;

    for byte in data {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }

    (b << 16) | a
}

impl AppManager {
    pub fn new_connected(app_id: AppId_t) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            env::set_var("SteamAppId", app_id.to_string());
        }

        #[cfg(feature = "cli")]
        let silent = false;
        #[cfg(feature = "gui")]
        let silent = true;

        let connected_steam = match ConnectedSteam::new(silent) {
            Ok(c) => c,
            Err(e) => {
                return Err(e);
            }
        };

        Ok(Self {
            app_id,
            connected_steam,
            definitions_loaded: false,
            user_stats_received: false,
            achievement_definitions: vec![],
            stat_definitions: vec![],
        })
    }

    fn request_current_stats(&mut self) -> Result<(), SamError> {
        if self.user_stats_received {
            return Ok(());
        }

        // Offline (or backend unreachable): Steam never services the
        // UserStatsReceived callback, so waiting would just stall for the full
        // timeout. Skip it and fall back to the on-disk stats cache so the app
        // still loads. Treat a failed BLoggedOn check as "assume online".
        if self.connected_steam.user.b_logged_on() == Ok(false) {
            eprintln!("[APP MANAGER] Steam is offline; loading from cached stats without a live request");
            return Ok(());
        }

        let steam_id = match self.connected_steam.user.get_steam_id() {
            Ok(id) => id,
            Err(e) => {
                eprintln!("[APP MANAGER] Error getting steam id: {}", e);
                return Err(SamError::UnknownError);
            }
        };

        dev_println!(
            "APPSRV",
            "Requesting current stats for current user: {:?}",
            steam_id
        );

        // A timeout here is non-fatal: proceed with whatever stats Steam has
        // cached rather than failing the whole load.
        match self.wait_for_user_stats(steam_id) {
            Ok(EResult::k_EResultOK) => self.user_stats_received = true,
            Ok(result) => {
                eprintln!("[APP MANAGER] RequestCurrentStats returned {result:?}; continuing with cached stats")
            }
            Err(SamError::Timeout) => {
                eprintln!("[APP MANAGER] RequestCurrentStats timed out; continuing with cached stats")
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    /// Request stats for `steam_id` (current user or any other) and block until
    /// Steam services the `UserStatsReceived_t` callback, returning its result
    /// code. Shared by the current-user path and the other-user lookups.
    fn wait_for_user_stats(&self, steam_id: CSteamID) -> Result<EResult, SamError> {
        let callback_handle = match self.connected_steam.user_stats.request_user_stats(steam_id) {
            Ok(callback_handle) => callback_handle,
            Err(e) => {
                eprintln!("[APP MANAGER] Error requesting user stats: {}", e);
                return Err(SamError::UnknownError);
            }
        };

        // Try for 30 seconds at ~60 fps. Bulk operations spawn many workers
        // that all queue user-stats requests through Steam's single IPC, so a
        // single request can wait a while before Steam services it.
        for _ in 0..1800 {
            let completed = match self
                .connected_steam
                .utils
                .is_api_call_completed(callback_handle)
            {
                Ok(res) => res,
                Err(e) => {
                    eprintln!(
                        "[APP MANAGER] Error checking request_user_stats api call completed: {}",
                        e
                    );
                    return Err(SamError::UnknownError);
                }
            };

            if completed {
                let result = match self
                    .connected_steam
                    .utils
                    .get_api_call_result::<UserStatsReceived_t>(
                        callback_handle,
                        SteamCallbackId::UserStatsReceived,
                    ) {
                    Ok(res) => res,
                    Err(e) => {
                        eprintln!(
                            "[APP MANAGER] Error getting request_user_stats api call result: {}",
                            e
                        );
                        return Err(SamError::UnknownError);
                    }
                };

                dev_println!("APPSRV", "User stats received callback result: {result:?}");
                return Ok(result.m_eResult);
            }

            std::thread::sleep(std::time::Duration::from_millis(17));
        }

        eprintln!("[APP MANAGER] Requesting user stats timed out");
        Err(SamError::Timeout)
    }

    /// Resolve a `friend` string — either a SteamID64 or a persona name from the
    /// current user's friends list — then read their unlock times for this app.
    pub fn fetch_friend_unlock_times(
        &mut self,
        friend: &str,
    ) -> Result<Vec<AchievementUnlock>, SamError> {
        let friend = friend.trim();
        // A bare SteamID64 is used directly; anything else is a persona name
        // looked up in the current user's localconfig.vdf friends block.
        let steam_id64 = match friend.parse::<u64>() {
            Ok(id) if id >= user_unlock_times::STEAMID64_BASE => id,
            _ => {
                let my_account = user_unlock_times::account_id(self.current_steam_id64()?);
                let cfg = user_unlock_times::localconfig_path(my_account)?;
                user_unlock_times::find_friend_steamid64(&cfg, friend).ok_or_else(|| {
                    eprintln!(
                        "[APP MANAGER] Friend '{friend}' not found in {}",
                        cfg.display()
                    );
                    SamError::UnknownError
                })?
            }
        };
        self.fetch_user_unlock_times(steam_id64)
    }

    /// SteamID64 of the currently logged-in user.
    pub fn current_steam_id64(&self) -> Result<u64, SamError> {
        self.connected_steam
            .user
            .get_steam_id()
            .map(|id| id.m_steamid)
            .map_err(|_| SamError::UnknownError)
    }

    /// Fetch another user's achievement unlock times for this app. Steam only
    /// writes an on-disk stats cache for accounts that have signed in on this
    /// machine, so locally-cached accounts get a single bulk parse while remote
    /// friends fall back to the per-user API (names from one bulk schema parse).
    pub fn fetch_user_unlock_times(
        &mut self,
        steam_id64: u64,
    ) -> Result<Vec<AchievementUnlock>, SamError> {
        let account_id = user_unlock_times::account_id(steam_id64);
        let steam_id = CSteamID {
            m_steamid: steam_id64,
        };

        // A locally-cached account (signed in on this machine) has its stats on
        // disk, so bulk-parse those directly — no live request, which also avoids
        // a spurious timeout masking data we already hold.
        let user_path = user_unlock_times::user_stats_file(account_id, self.app_id)?;
        if user_path.exists() {
            return user_unlock_times::read_unlock_times(account_id, self.app_id);
        }

        // No local cache: depend on the live request, so a non-OK result means
        // the target's game details / achievements are private.
        let result = self.wait_for_user_stats(steam_id)?;
        if result != EResult::k_EResultOK {
            eprintln!("[APP MANAGER] RequestUserStats for {steam_id64} returned {result:?}");
            return Err(SamError::ProfilePrivate);
        }

        let names = user_unlock_times::read_schema_achievements(self.app_id)?;
        let mut out = Vec::with_capacity(names.len());
        for (api_name, display_name) in names {
            let (achieved, unlock_time) = self
                .connected_steam
                .user_stats
                .get_user_achievement_and_unlock_time(steam_id, &api_name)
                .unwrap_or((false, 0));
            out.push(AchievementUnlock {
                api_name,
                display_name,
                achieved,
                unlock_time: if achieved && unlock_time > 0 {
                    Some(unlock_time)
                } else {
                    None
                },
            });
        }
        Ok(out)
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs
    pub fn load_definitions(&mut self) -> Result<(), SamError> {
        self.request_current_stats()?;
        let steam_locator_lock = SteamLocator::global();
        let steam_locator = steam_locator_lock.read().unwrap();

        let bin_file = match steam_locator.get_user_game_stats_schema(&self.app_id) {
            Ok(bin_file) => bin_file,
            Err(e) => {
                eprintln!("[APP MANAGER] Error getting user game stats file: {}", e);
                return Err(e);
            }
        };

        #[cfg(debug_assertions)]
        {
            match std::fs::read(&bin_file) {
                Ok(bytes) => {
                    dev_println!(
                        "APPMAN",
                        "Loading user game stats file {} (Checksum: {:08x})",
                        bin_file.display(),
                        adler32(&bytes)
                    );
                }
                Err(e) => {
                    dev_println!("APPMAN", "Error loading user game stats file: {}", e);
                }
            };
        }

        let kv = match KeyValue::load_as_binary(&bin_file) {
            Ok(kv) => kv,
            Err(e) => {
                eprintln!(
                    "[APP MANAGER] Error loading key value from path {}: {:?}",
                    bin_file.display(),
                    e
                );
                return Err(SamError::UnknownError);
            }
        };

        let current_language = self.connected_steam.apps.get_current_game_language();
        let stats = kv.get(&self.app_id.to_string());
        let stats = stats.get("stats");

        let mut stat_definitions: Vec<StatDefinition> = vec![];
        let mut achievement_definitions: Vec<AchievementDefinition> = vec![];

        for (_, stat) in stats.children.iter() {
            if !stat.valid {
                continue;
            }

            let mut type_ = UserStatType::Invalid;

            // Schema in the new format?
            let type_node = stat.get("type");
            if let KeyValueData::String(ref type_str) = type_node.data {
                if let Ok(parsed) = type_str.parse::<UserStatType>() {
                    type_ = parsed;
                }
            }

            // Schema in the old format?
            if type_ == UserStatType::Invalid {
                let type_int_node = stat.get("type_int");

                let raw_type = if type_int_node.valid {
                    type_int_node.as_i32(0)
                } else {
                    type_node.as_i32(0)
                };

                type_ = UserStatType::try_from(raw_type as u8)
                    .unwrap_or_else(|_| UserStatType::Invalid);
            }

            match type_ {
                UserStatType::Invalid => {
                    eprintln!("[APP MANAGER] Failed to parse user stat type: {type_node:?}");
                    continue;
                }

                UserStatType::Integer => {
                    let id = stat.get("name").as_string("");
                    let name = Self::get_localized_string(
                        stat.get("display").get("name"),
                        &current_language,
                        &id,
                    );
                    stat_definitions.push(StatDefinition::Integer(IntegerStatDefinition {
                        base: BaseStatDefinition {
                            id: stat.get("name").as_string(""),
                            display_name: name,
                            permission: stat.get("permission").as_i32(0),
                            app_id: self.app_id,
                        },
                        min_value: stat.get("min").as_i32(i32::MIN),
                        max_value: stat.get("max").as_i32(i32::MAX),
                        max_change: stat.get("maxchange").as_i32(0),
                        increment_only: stat.get("incrementonly").as_bool(false),
                        default_value: stat.get("default").as_i32(0),
                        set_by_trusted_game_server: stat.get("bSetByTrustedGS").as_bool(false),
                    }));
                }

                UserStatType::Float | UserStatType::AverageRate => {
                    let id = stat.get("name").as_string("");
                    let name = Self::get_localized_string(
                        stat.get("display").get("name"),
                        &current_language,
                        &id,
                    );
                    stat_definitions.push(StatDefinition::Float(FloatStatDefinition {
                        base: BaseStatDefinition {
                            id: stat.get("name").as_string(""),
                            display_name: name,
                            permission: stat.get("permission").as_i32(0),
                            app_id: self.app_id,
                        },
                        min_value: stat.get("min").as_f32(f32::MIN),
                        max_value: stat.get("max").as_f32(f32::MAX),
                        max_change: stat.get("maxchange").as_f32(0f32),
                        increment_only: stat.get("incrementonly").as_bool(false),
                        default_value: stat.get("default").as_f32(0f32),
                    }));
                }

                UserStatType::Achievements | UserStatType::GroupAchievements => {
                    for bits in stat.children.iter() {
                        if bits.0.to_lowercase() != "bits" {
                            continue;
                        }

                        if !bits.1.valid || bits.1.children.is_empty() {
                            dev_println!("APPMAN", "Invalid achievements bits.1: {:?}", bits.1);
                            continue;
                        }

                        for bit in bits.1.children.iter() {
                            let id = bit.1.get("name").as_string("");
                            let name = Self::get_localized_string(
                                bit.1.get("display").get("name"),
                                &current_language,
                                &id,
                            );
                            let description = Self::get_localized_string(
                                bit.1.get("display").get("desc"),
                                &current_language,
                                "",
                            );

                            achievement_definitions.push(AchievementDefinition {
                                id,
                                app_id: self.app_id,
                                name,
                                description,
                                icon_normal: format!("https://cdn.steamstatic.com/steamcommunity/public/images/apps/{}/{}", self.app_id, bit.1.get("display").get("icon").as_string("")),
                                icon_locked: format!("https://cdn.steamstatic.com/steamcommunity/public/images/apps/{}/{}", self.app_id, bit.1.get("display").get("icon_gray").as_string("")),
                                is_hidden: bit.1.get("display").get("hidden").as_bool(false),
                                permission: bit.1.get("permission").as_i32(0),
                            })
                        }
                    }
                }
            }
        }

        self.stat_definitions = stat_definitions;
        self.achievement_definitions = achievement_definitions;
        self.definitions_loaded = true;

        Ok(())
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs#L420
    pub fn get_achievements(
        &mut self,
        with_global_achieved: bool,
    ) -> Result<Vec<AchievementInfo>, SamError> {
        let mut global_stats_fetched = EResult::k_EResultFail;
        if with_global_achieved {
            let callback_handle = self
                .connected_steam
                .user_stats
                .request_global_achievement_percentages()
                .map_err(|_| SamError::UnknownError)?;

            // Try for 10 seconds at 60 fps
            for _ in 0..600 {
                if self
                    .connected_steam
                    .utils
                    .is_api_call_completed(callback_handle)
                    .map_err(|_| SamError::UnknownError)?
                {
                    let result = self
                        .connected_steam
                        .utils
                        .get_api_call_result::<GlobalAchievementPercentagesReady_t>(
                            callback_handle,
                            SteamCallbackId::GlobalAchievementPercentagesReady,
                        )
                        .map_err(|_| SamError::UnknownError)?;
                    global_stats_fetched = result.m_eResult;
                    dev_println!(
                        "APPSRV",
                        "Global achievement percentages callback result: {result:?}"
                    );
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(17));
            }
        }

        let mut achievement_infos: Vec<AchievementInfo> = vec![];

        if !self.definitions_loaded {
            self.load_definitions()?;
        }

        for def in self.achievement_definitions.iter() {
            if def.id.is_empty() {
                dev_println!("APPMAN", "Achievement definition ID is empty:");
                dev_println!("{def:?}");
                continue;
            }

            let def_id = &def.id;
            match self
                .connected_steam
                .user_stats
                .get_achievement_and_unlock_time(def_id)
            {
                Ok((is_achieved, unlock_time)) => {
                    let global_achieved_percent = if global_stats_fetched == EResult::k_EResultFail
                    {
                        None
                    } else {
                        match self
                            .connected_steam
                            .user_stats
                            .get_achievement_achieved_percent(def_id)
                        {
                            Ok(percent) => Some(percent),
                            Err(_) => {
                                dev_println!(
                                    "APPSRV",
                                    "Failed to get achievement percent for achievement: {def_id}"
                                );
                                None
                            }
                        }
                    };

                    achievement_infos.push(AchievementInfo {
                        id: def_id.to_string(),
                        is_achieved,
                        unlock_time: if is_achieved && unlock_time > 0 {
                            UNIX_EPOCH
                                .checked_add(std::time::Duration::from_secs(unlock_time as u64))
                        } else {
                            None
                        },
                        icon_normal: def.icon_normal.clone(),
                        icon_locked: if def.icon_locked.is_empty() {
                            def.icon_normal.clone()
                        } else {
                            def.icon_locked.clone()
                        },
                        permission: def.permission,
                        name: def.name.clone(),
                        description: def.description.clone(),
                        global_achieved_percent,
                    });
                }
                Err(_) => {
                    dev_println!(
                        "APPSRV",
                        "Failed to get achievement info for achievement: {def_id}"
                    );
                    continue;
                }
            }
        }

        dev_println!(
            "APPMAN",
            "Loaded {} achievement definitions for {} achievements for app {}",
            self.achievement_definitions.len(),
            achievement_infos.len(),
            self.app_id
        );

        Ok(achievement_infos)
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs#L519
    pub fn get_statistics(&mut self) -> Result<Vec<StatInfo>, SamError> {
        let mut statistics_info: Vec<StatInfo> = vec![];

        if !self.definitions_loaded {
            self.load_definitions()?;
        }

        for stat in self.stat_definitions.iter() {
            match stat {
                StatDefinition::Float(definition) => {
                    if definition.base.id.is_empty() {
                        continue;
                    }

                    let stat_value = match self
                        .connected_steam
                        .user_stats
                        .get_stat_float(&definition.base.id)
                    {
                        Ok(value) => {
                            if value.is_nan() {
                                dev_println!(
                                    "APPMAN",
                                    "Converting NAN stat float value to 0: {}",
                                    &definition.base.id
                                );
                                0f32
                            } else {
                                value
                            }
                        }
                        Err(_) => {
                            let stat_id = definition.base.id.to_string();
                            dev_println!(
                                "APPSRV",
                                "Failed to get float stat info for stat: {stat_id}"
                            );
                            continue;
                        }
                    };

                    statistics_info.push(StatInfo::Float(FloatStatInfo {
                        id: definition.base.id.clone(),
                        app_id: definition.base.app_id,
                        display_name: definition.base.display_name.clone(),
                        float_value: stat_value,
                        original_value: stat_value,
                        is_increment_only: definition.increment_only,
                        permission: definition.base.permission,
                        min_value: definition.min_value,
                        max_value: definition.max_value,
                    }));
                }

                StatDefinition::Integer(definition) => {
                    if definition.base.id.is_empty() {
                        continue;
                    }

                    let stat_value = match self
                        .connected_steam
                        .user_stats
                        .get_stat_i32(&definition.base.id)
                    {
                        Ok(value) => value,
                        Err(_) => {
                            let stat_id = definition.base.id.to_string();
                            dev_println!(
                                "APPSRV",
                                "Failed to get int stat info for stat: {stat_id}"
                            );
                            continue;
                        }
                    };

                    statistics_info.push(StatInfo::Integer(IntStatInfo {
                        id: definition.base.id.clone(),
                        app_id: definition.base.app_id,
                        display_name: definition.base.display_name.clone(),
                        int_value: stat_value,
                        original_value: stat_value,
                        is_increment_only: definition.increment_only,
                        permission: definition.base.permission,
                        min_value: definition.min_value,
                        max_value: definition.max_value,
                    }));
                }
            };
        }

        Ok(statistics_info)
    }

    pub fn set_achievement(
        &self,
        achievement_id: &str,
        unlock: bool,
        store: bool,
    ) -> Result<bool, SamError> {
        if unlock {
            match self
                .connected_steam
                .user_stats
                .set_achievement(achievement_id)
            {
                Ok(_) => {
                    if store {
                        return self
                            .connected_steam
                            .user_stats
                            .store_stats()
                            .map_err(|_| SamError::StatStoreFailed);
                    }
                    Ok(true)
                }
                Err(_) => Err(SamError::LockUnlockAchievementFailed),
            }
        } else {
            match self
                .connected_steam
                .user_stats
                .clear_achievement(achievement_id)
            {
                Ok(_) => {
                    if store {
                        return self
                            .connected_steam
                            .user_stats
                            .store_stats()
                            .map_err(|_| SamError::StatStoreFailed);
                    }
                    Ok(true)
                }
                Err(_) => Err(SamError::LockUnlockAchievementFailed),
            }
        }
    }

    pub fn store_stats_and_achievements(&self) -> Result<(), SamError> {
        self.connected_steam
            .user_stats
            .store_stats()
            .map_err(|_| SamError::StatStoreFailed)?;
        Ok(())
    }

    pub fn read_int_stat_state(&self, id: &str) -> StatState<i32> {
        let (min, max, increment_only, default) = self
            .stat_definitions
            .iter()
            .find_map(|d| match d {
                StatDefinition::Integer(def) if def.base.id == id => Some(def),
                _ => None,
            })
            .map(|d| (d.min_value, d.max_value, d.increment_only, d.default_value))
            .unwrap_or((i32::MIN, i32::MAX, false, 0));
        let current = self.connected_steam.user_stats.get_stat_i32(id).ok();
        StatState {
            min,
            max,
            increment_only,
            default,
            current,
        }
    }

    pub fn read_float_stat_state(&self, id: &str) -> StatState<f32> {
        let (min, max, increment_only, default) = self
            .stat_definitions
            .iter()
            .find_map(|d| match d {
                StatDefinition::Float(def) if def.base.id == id => Some(def),
                _ => None,
            })
            .map(|d| (d.min_value, d.max_value, d.increment_only, d.default_value))
            .unwrap_or((f32::MIN, f32::MAX, false, 0.0));
        let current = self.connected_steam.user_stats.get_stat_float(id).ok();
        StatState {
            min,
            max,
            increment_only,
            default,
            current,
        }
    }

    pub fn unlock_all_achievements(&mut self) -> Result<(), SamError> {
        let achievements = self.get_achievements(false)?;
        let mut has_failures = false;
        for achievement in achievements {
            if achievement.is_achieved {
                continue;
            }

            if achievement.permission != 0 {
                continue;
            }

            match self
                .connected_steam
                .user_stats
                .set_achievement(achievement.id.as_str())
            {
                Ok(_) => {}
                Err(_) => {
                    eprintln!(
                        "[APP MANAGER] Failed to unlock achievement for app {} while unlocking all: {achievement:?}",
                        self.app_id
                    );
                    has_failures = true;
                }
            }
        }

        self.connected_steam
            .user_stats
            .store_stats()
            .map_err(|_| SamError::StatStoreFailed)?;

        if has_failures {
            Err(SamError::LockUnlockAchievementFailed)
        } else {
            Ok(())
        }
    }

    pub fn set_stat_i32(&self, stat_name: &str, stat_value: i32) -> Result<bool, SamError> {
        match self
            .connected_steam
            .user_stats
            .set_stat_i32(stat_name, stat_value)
        {
            Ok(_) => self
                .connected_steam
                .user_stats
                .store_stats()
                .map_err(|_| SamError::StatStoreFailed),
            Err(_) => Err(SamError::UnknownError),
        }
    }

    pub fn set_stat_f32(&self, stat_name: &str, stat_value: f32) -> Result<bool, SamError> {
        match self
            .connected_steam
            .user_stats
            .set_stat_float(stat_name, stat_value)
        {
            Ok(_) => self
                .connected_steam
                .user_stats
                .store_stats()
                .map_err(|_| SamError::StatStoreFailed),
            Err(_) => Err(SamError::UnknownError),
        }
    }

    pub fn reset_all_stats(&self, achievements_too: bool) -> Result<bool, SamError> {
        match self
            .connected_steam
            .user_stats
            .reset_all_stats(achievements_too)
        {
            Ok(_) => self
                .connected_steam
                .user_stats
                .store_stats()
                .map_err(|_| SamError::StatStoreFailed),
            Err(_) => Err(SamError::UnknownError),
        }
    }

    fn get_localized_string(kv: &KeyValue, language: &str, default_value: &str) -> String {
        let name = kv.get(language).as_string("");
        if !name.is_empty() {
            return name;
        }

        if language != "english" {
            let name = kv.get("english").as_string("");
            if !name.is_empty() {
                return name;
            }
        }

        let name = kv.as_string("");
        if !name.is_empty() {
            return name;
        }

        default_value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::app_manager::adler32;

    #[test]
    fn test_adler32() {
        println!("Adler null: {:08x}", adler32(&vec![]));
    }
}
