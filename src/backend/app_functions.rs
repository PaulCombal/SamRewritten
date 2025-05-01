use std::env;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::key_value::KeyValue;
use crate::backend::stat_definitions::{AchievementDefinition, AchievementInfo, BaseStatDefinition, FloatStatDefinition, FloatStatInfo, IntStatInfo, IntegerStatDefinition, StatDefinition, StatInfo};
use crate::backend::types::UserStatType;
use crate::dev_println;

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

// Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs
pub fn load_user_game_stats_schema(app_id: u32, connected_steam: &ConnectedSteam) -> (Vec<AchievementDefinition>, Vec<StatDefinition>) {
    let home = env::var("HOME").expect("HOME not set");
    let bin_file = PathBuf::from(home + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_" + &app_id.to_string() + ".bin");
    let kv = KeyValue::load_as_binary(bin_file).expect("Failed to load KeyValue");
    let current_language = connected_steam.apps.get_current_game_language();
    let stats = kv.get(&app_id.to_string());
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
        
        let type_ = UserStatType::try_from(raw_type as u8).unwrap();
        
        match type_ { 
            UserStatType::Invalid => {
                continue;
            }
            
            UserStatType::Integer => {
                let id = stat.get("name").as_string("");
                let name = get_localized_string(stat.get("display").get("name"), &current_language, &id);
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
                let name = get_localized_string(stat.get("display").get("name"), &current_language, &id);
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
                        let name = get_localized_string(bit.1.get("display").get("name"), &current_language, &id);
                        let description = get_localized_string(bit.1.get("display").get("desc"), &current_language, "");

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
    
    (achievement_definitions, stat_definitions)
}

// Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs#L420
pub fn get_achievements(app_id: u32, connected_steam: &ConnectedSteam) -> Vec<AchievementInfo> {
    let (achievement_definitions, _) = load_user_game_stats_schema(app_id, connected_steam);
    let mut achievement_infos: Vec<AchievementInfo> = vec![];
    
    for def in achievement_definitions.iter() {
        if def.id.is_empty() { 
            continue;
        }
        
        let def_id = &def.id;
        match connected_steam.user_stats.get_achievement_and_unlock_time(def_id) {
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
    
    achievement_infos
}

// Reference: https://github.com/gibbed/SteamAchievementManager/blob/master/SAM.Game/Manager.cs#L519
pub fn get_statistics(app_id: u32, connected_steam: &ConnectedSteam) -> Vec<StatInfo> {
    let (_, stat_definitions) = load_user_game_stats_schema(app_id, connected_steam);
    let mut statistics_info: Vec<StatInfo> = vec![];

    for stat in stat_definitions.iter() {
        match stat {
            StatDefinition::Float(definition) => {
                if definition.base.id.is_empty() {
                    continue;
                }

                let stat_value = match connected_steam.user_stats.get_stat_float(&definition.base.id) {
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

                let stat_value = match connected_steam.user_stats.get_stat_i32(&definition.base.id) {
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

    statistics_info
}