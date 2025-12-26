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

use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::key_value::KeyValue;
use crate::backend::stat_definitions::{
    AchievementDefinition, AchievementInfo, BaseStatDefinition, FloatStatDefinition, FloatStatInfo,
    IntStatInfo, IntegerStatDefinition, StatDefinition, StatInfo,
};
use crate::backend::types::UserStatType;
use crate::dev_println;
use crate::steam_client::steamworks_types::{
    AppId_t, EResult, GlobalAchievementPercentagesReady_t, UserStatsReceived_t,
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

impl<'a> AppManager {
    pub fn new_connected(app_id: AppId_t) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            env::set_var("SteamAppId", app_id.to_string());
        }

        #[cfg(feature = "cli")]
        let silent = false;
        #[cfg(not(feature = "cli"))]
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

        let steam_id = self
            .connected_steam
            .user
            .get_steam_id()
            .map_err(|_| SamError::UnknownError)?;
        dev_println!(
            "[APP SERVER] Requesting current stats for current user: {:?}",
            steam_id
        );
        let callback_handle = self
            .connected_steam
            .user_stats
            .request_user_stats(steam_id)
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
                    .get_api_call_result::<UserStatsReceived_t>(
                        callback_handle,
                        SteamCallbackId::UserStatsReceived,
                    )
                    .map_err(|_| SamError::UnknownError)?;

                dev_println!("[APP SERVER] User stats received callback result: {result:?}");

                if result.m_eResult == EResult::k_EResultOK {
                    self.user_stats_received = true;
                }

                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(17));
        }

        Ok(())
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs
    pub fn load_definitions(&mut self) -> Result<(), SamError> {
        self.request_current_stats()?;
        let steam_locator_lock = SteamLocator::global();
        let mut steam_locator = steam_locator_lock.write().unwrap();
        let bin_file = steam_locator.get_user_game_stats_schema(&self.app_id)?;

        let kv = KeyValue::load_as_binary(bin_file).map_err(|_| SamError::UnknownError)?;
        let current_language = self.connected_steam.apps.get_current_game_language();
        let stats = kv.get(&self.app_id.to_string());
        let stats = stats.get("stats");

        let mut stat_definitions: Vec<StatDefinition> = vec![];
        let mut achievement_definitions: Vec<AchievementDefinition> = vec![];

        for (_, stat) in stats.children.iter() {
            if !stat.valid {
                continue;
            }

            let raw_type = if stat.get("type_int").valid {
                stat.get("type_int").as_i32(0)
            } else {
                stat.get("type").as_i32(0)
            };

            let type_ =
                UserStatType::try_from(raw_type as u8).map_err(|_| SamError::UnknownError)?;

            match type_ {
                UserStatType::Invalid => {
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

                        if bits.1.valid == false || bits.1.children.is_empty() {
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
    pub fn get_achievements(&mut self) -> Result<Vec<AchievementInfo>, SamError> {
        let callback_handle = self
            .connected_steam
            .user_stats
            .request_global_achievement_percentages()
            .map_err(|_| SamError::UnknownError)?;
        let mut global_stats_fetched = EResult::k_EResultFail;

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
                    "[APP SERVER] Global achievement percentages callback result: {result:?}"
                );
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(17));
        }

        let mut achievement_infos: Vec<AchievementInfo> = vec![];

        if !self.definitions_loaded {
            self.load_definitions()?;
        }

        for def in self.achievement_definitions.iter() {
            if def.id.is_empty() {
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
                                    "[APP SERVER] Failed to get achievement percent for achievement: {def_id}"
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
                        icon_normal: def.clone().icon_normal,
                        icon_locked: if def.icon_locked.is_empty() {
                            def.clone().icon_normal
                        } else {
                            def.clone().icon_locked
                        },
                        permission: def.clone().permission,
                        name: def.clone().name,
                        description: def.clone().description,
                        global_achieved_percent,
                    });
                }
                Err(_) => {
                    dev_println!(
                        "[APP SERVER] Failed to get achievement info for achievement: {def_id}"
                    );
                    continue;
                }
            }
        }

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
                                dev_println!("[APP MANAGER] Converting NAN stat float value to 0: {}", &definition.base.id);
                                0f32
                            } else {
                                value
                            }
                        },
                        Err(_) => {
                            let stat_id = definition.base.id.to_string();
                            dev_println!(
                                "[APP SERVER] Failed to get float stat info for stat: {stat_id}"
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
                                "[APP SERVER] Failed to get int stat info for stat: {stat_id}"
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
                    }));
                }
            };
        }

        Ok(statistics_info)
    }

    pub fn set_achievement(&self, achievement_id: &str, unlock: bool) -> Result<bool, SamError> {
        if unlock {
            match self
                .connected_steam
                .user_stats
                .set_achievement(achievement_id)
            {
                Ok(_) => self
                    .connected_steam
                    .user_stats
                    .store_stats()
                    .map_err(|_| SamError::StatStoreFailed),
                Err(_) => Err(SamError::UnknownError),
            }
        } else {
            match self
                .connected_steam
                .user_stats
                .clear_achievement(achievement_id)
            {
                Ok(_) => self
                    .connected_steam
                    .user_stats
                    .store_stats()
                    .map_err(|_| SamError::StatStoreFailed),
                Err(_) => Err(SamError::UnknownError),
            }
        }
    }

    pub fn unlock_all_achievements(&mut self) -> Result<(), SamError> {
        let achievements = self.get_achievements()?;
        for achievement in achievements {
            if achievement.is_achieved {
                continue;
            }

            if (achievement.permission & 2) != 0 {
                continue;
            }

            match self
                .connected_steam
                .user_stats
                .set_achievement(achievement.id.as_str())
            {
                Ok(_) => {}
                Err(_) => return Err(SamError::UnknownError),
            }
        }

        self.connected_steam
            .user_stats
            .store_stats()
            .map_err(|_| SamError::StatStoreFailed)?;

        Ok(())
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
