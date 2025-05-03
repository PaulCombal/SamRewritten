use std::env;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::key_value::KeyValue;
use crate::backend::stat_definitions::{AchievementDefinition, AchievementInfo, BaseStatDefinition, FloatStatDefinition, FloatStatInfo, IntStatInfo, IntegerStatDefinition, StatDefinition, StatInfo};
use crate::backend::types::UserStatType;
use crate::dev_println;
use crate::steam_client::types::{AppId_t};

pub struct AppManager<'a> {
    app_id: AppId_t,
    connected_steam: ConnectedSteam<'a>,
    definitions_loaded: bool,
    achievement_definitions: Vec<AchievementDefinition>,
    stat_definitions: Vec<StatDefinition>,
}

impl<'a> AppManager<'a> {
    pub fn new_connected(app_id: AppId_t) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            env::set_var("SteamAppId", app_id.to_string());
        }

        let connected_steam = match ConnectedSteam::new() {
            Ok(c) => c,
            Err(e) => { return Err(e); }
        };
        
        Ok(Self {
            app_id,
            connected_steam,
            definitions_loaded: false,
            achievement_definitions: vec![],
            stat_definitions: vec![],
        })
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs
    pub fn load_definitions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        let home = env::var("HOME")?;
        #[cfg(target_os = "linux")]
        let bin_file = PathBuf::from(home + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_" + &self.app_id.to_string() + ".bin");
        #[cfg(target_os = "windows")]
        let program_files = env::var("ProgramFiles(x86)")?;
        #[cfg(target_os = "windows")]
        let bin_file = PathBuf::from(program_files + "\\Steam\\appcache\\stats\\UserGameStatsSchema_" + &app_id.to_string() + ".bin");

        let kv = KeyValue::load_as_binary(bin_file)?;
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

            let type_ = UserStatType::try_from(raw_type as u8)?;

            match type_ {
                UserStatType::Invalid => {
                    continue;
                }

                UserStatType::Integer => {
                    let id = stat.get("name").as_string("");
                    let name = Self::get_localized_string(stat.get("display").get("name"), &current_language, &id);
                    stat_definitions.push(StatDefinition::Integer(IntegerStatDefinition {
                        base: BaseStatDefinition {
                            id: stat.get("name").as_string(""),
                            display_name: name,
                            permission: stat.get("permission").as_i32(0)
                        },
                        min_value: stat.get("min").as_i32(i32::MIN),
                        max_value: stat.get("max").as_i32(i32::MAX),
                        max_change: stat.get("maxchange").as_i32(0),
                        increment_only: stat.get("incrementonly").as_bool(false),
                        default_value: stat.get("default").as_i32(0),
                        set_by_trusted_game_server: stat.get("bSetByTrustedGS").as_bool(false),
                    })
                    );
                }

                UserStatType::Float | UserStatType::AverageRate => {
                    let id = stat.get("name").as_string("");
                    let name = Self::get_localized_string(stat.get("display").get("name"), &current_language, &id);
                    stat_definitions.push(StatDefinition::Float(FloatStatDefinition {
                        base: BaseStatDefinition {
                            id: stat.get("name").as_string(""),
                            display_name: name,
                            permission: stat.get("permission").as_i32(0)
                        },
                        min_value: stat.get("min").as_f32(f32::MIN),
                        max_value: stat.get("max").as_f32(f32::MAX),
                        max_change: stat.get("maxchange").as_f32(0f32),
                        increment_only: stat.get("incrementonly").as_bool(false),
                        default_value: stat.get("default").as_f32(0f32),
                    })
                    );
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
                            let name = Self::get_localized_string(bit.1.get("display").get("name"), &current_language, &id);
                            let description = Self::get_localized_string(bit.1.get("display").get("desc"), &current_language, "");

                            achievement_definitions.push(AchievementDefinition {
                                id,
                                name,
                                description,
                                icon_normal: bit.1.get("display").get("icon").as_string(""),
                                icon_locked: bit.1.get("display").get("icon_gray").as_string(""),
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
    pub fn get_achievements(&mut self) -> Result<Vec<AchievementInfo>, Box<dyn std::error::Error>> {
        if !self.definitions_loaded {
            self.load_definitions()?;
        }
        
        let mut achievement_infos: Vec<AchievementInfo> = vec![];

        for def in self.achievement_definitions.iter() {
            if def.id.is_empty() {
                continue;
            }

            let def_id = &def.id;
            match self.connected_steam.user_stats.get_achievement_and_unlock_time(def_id) {
                Ok((is_achieved, unlock_time)) => {
                    achievement_infos.push(AchievementInfo {
                        id: def_id.to_string(),
                        is_achieved,
                        unlock_time: if is_achieved && unlock_time > 0 { UNIX_EPOCH.checked_add(std::time::Duration::from_secs(unlock_time as u64)) } else { None },
                        icon_normal: def.clone().icon_normal,
                        icon_locked: if def.icon_locked.is_empty() { def.clone().icon_normal } else { def.clone().icon_locked },
                        permission: def.clone().permission,
                        name: def.clone().name,
                        description: def.clone().description
                    });
                },
                Err(_) => {
                    dev_println!("[APP SERVER] Failed to get achievement info for achievement: {def_id}");
                    continue;
                }
            }
        }

        Ok(achievement_infos)
    }

    // Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs#L519
    pub fn get_statistics(&mut self) -> Result<Vec<StatInfo>, Box<dyn std::error::Error>> {
        if !self.definitions_loaded {
            self.load_definitions()?;
        }
        
        let mut statistics_info: Vec<StatInfo> = vec![];

        for stat in self.stat_definitions.iter() {
            match stat {
                StatDefinition::Float(definition) => {
                    if definition.base.id.is_empty() {
                        continue;
                    }

                    let stat_value = match self.connected_steam.user_stats.get_stat_float(&definition.base.id) {
                        Ok(value) => value,
                        Err(_) => {
                            let stat_id = definition.base.id.to_string();
                            dev_println!("[APP SERVER] Failed to get float stat info for stat: {stat_id}");
                            continue;
                        }
                    };

                    statistics_info.push(StatInfo::Float(FloatStatInfo {
                        id: definition.base.id.clone(),
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

                    let stat_value = match self.connected_steam.user_stats.get_stat_i32(&definition.base.id) {
                        Ok(value) => value,
                        Err(_) => {
                            let stat_id = definition.base.id.to_string();
                            dev_println!("[APP SERVER] Failed to get int stat info for stat: {stat_id}");
                            continue;
                        }
                    };

                    statistics_info.push(StatInfo::Integer(IntStatInfo {
                        id: definition.base.id.clone(),
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

    pub fn set_achievement(&self, achievement_id: &str, unlock: bool) -> Result<(), Box<dyn std::error::Error>> {
        if unlock {
            match self.connected_steam.user_stats.set_achievement(achievement_id) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        else {
            match self.connected_steam.user_stats.clear_achievement(achievement_id) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }

    pub fn set_stat_i32(&self, stat_name: &str, stat_value: i32) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Check if we can circumvent increment_only by using a loop
        // I remember that increment_only only allows to increment by 1, but I can't find any trace
        match self.connected_steam.user_stats.set_stat_i32(stat_name, stat_value) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_stat_f32(&self, stat_name: &str, stat_value: f32) -> Result<(), Box<dyn std::error::Error>> {
        match self.connected_steam.user_stats.set_stat_float(stat_name, stat_value) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    
    pub fn get_app_id(&self) -> AppId_t {
        self.app_id
    }
    
    pub fn definitions_loaded(&self) -> bool {
        self.definitions_loaded
    }
    
    pub fn get_stat_definitions(&self) -> &Vec<StatDefinition> {
        &self.stat_definitions
    }
    
    pub fn get_achievement_definitions(&self) -> &Vec<AchievementDefinition> {
        &self.achievement_definitions
    }
    
    pub fn disconnect(&self) {
        self.connected_steam.shutdown();
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