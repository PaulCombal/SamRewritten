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

use std::fmt::Display;
use std::{fs};
use std::fs::File;
use std::io::{BufReader};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use crate::dev_println;
use crate::steam_client::steam_apps_001_wrapper::{SteamApps001, SteamApps001AppDataKeys};
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::utils::get_app_cache_dir;

pub struct AppLister<'a> {
    app_list_url: String,
    app_list_local: String,
    current_language: String,
    steam_apps_001: &'a SteamApps001,
    steam_apps: &'a SteamApps,
}

#[derive(Serialize, Deserialize)]
pub struct AppModel {
    pub app_id: AppId_t,
    pub app_name: String,
    pub image_url: Option<String>,
    pub app_type: AppModelType,
    pub developer: String,
    pub metacritic_score: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum AppModelType {
    App,
    Mod,
    Demo,
    Junk
}

impl Display for AppModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppModelType::App => write!(f, "App"),
            AppModelType::Mod => write!(f, "Mod"),
            AppModelType::Demo => write!(f, "Demo"),
            AppModelType::Junk => write!(f, "Junk"),
        }
    }
}

impl FromStr for AppModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "app" => Ok(AppModelType::App),
            "mod" => Ok(AppModelType::Mod),
            "demo" => Ok(AppModelType::Demo),
            "junk" => Ok(AppModelType::Junk),
            _ => Err(format!("'{}' is not a valid AppModelType", s)),
        }
    }
}

#[derive(Deserialize)]
pub struct XmlGame {
    #[serde(rename = "$text")]
    pub app_id: u32,
    #[serde(rename = "@type")]
    pub app_type: Option<String>
}

#[derive(Deserialize)]
struct XmlGames {
    #[serde(rename = "game")]
    pub games: Vec<XmlGame>,
}
impl<'a> AppLister<'a> {
    pub fn new(steam_apps_001: &'a SteamApps001, steam_apps: &'a SteamApps ) -> Self {
        let cache_dir = get_app_cache_dir();
        let app_list_url = std::env::var("APP_LIST_URL").unwrap_or(String::from("https://gib.me/sam/games.xml"));
        let app_list_local = std::env::var("APP_LIST_LOCAL").unwrap_or(String::from("/apps.xml"));
        let current_language = steam_apps.get_current_game_language();

        AppLister {
            app_list_url,
            app_list_local: cache_dir + &app_list_local,
            current_language,
            steam_apps_001,
            steam_apps,
        }
    }

    fn download_app_list_str(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(&self.app_list_url)?.text()?;
        Ok(response)
    }

    fn load_app_list_file(&self) -> Result<XmlGames, Box<dyn std::error::Error>> {
        let f = File::open(&self.app_list_local)?;
        let f = BufReader::new(f);
        let xml_data: XmlGames = quick_xml::de::from_reader(f)?;
        Ok(xml_data)
    }

    fn load_app_list_str(&self, source: &String) -> Result<XmlGames, Box<dyn std::error::Error>> {
        let xml_data: XmlGames = quick_xml::de::from_str(source)?;
        Ok(xml_data)
    }

    fn get_xml_games(&self) -> Result<XmlGames, Box<dyn std::error::Error>> {
        let should_update = match fs::metadata(&self.app_list_local) {
            Ok(metadata) => {
                let last_update = metadata.modified()?;
                let one_week_ago = SystemTime::now() - Duration::from_secs(7 * 24 * 60 * 60); // 7 days
                last_update < one_week_ago
            },
            Err(_) => true,
        };

        let xml_games : XmlGames;

        if should_update {
            let app_list_str = self.download_app_list_str()?;
            xml_games = self.load_app_list_str(&app_list_str)?;
            fs::write(&self.app_list_local, &app_list_str)?;

            dev_println!("Loaded app list from url");
        }
        else {
            dev_println!("Loading from local location");
            xml_games = self.load_app_list_file()?;

            dev_println!("Loaded app list from file");
        }

        Ok(xml_games)
    }

    fn get_app_image_url(&self, app_id: &AppId_t) -> Option<String>
    {
        let candidate = self.steam_apps_001.get_app_data(app_id, &SteamApps001AppDataKeys::SmallCapsule(&self.current_language).as_string()).unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"));
        }

        if self.current_language != "english" {
            let candidate = self.steam_apps_001.get_app_data(app_id, &SteamApps001AppDataKeys::SmallCapsule("english").as_string()).unwrap_or("".to_owned());
            if !candidate.is_empty() {
                return Some(format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"));
            }
        }

        let candidate = self.steam_apps_001.get_app_data(app_id, &SteamApps001AppDataKeys::Logo.as_string()).unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!("https://cdn.steamstatic.com/steamcommunity/public/images/apps/{app_id}/{candidate}.jpg"));
        }
        
        dev_println!("[ORCHESTRATOR] Failed to find image for app {}", app_id);

        None
    }

    pub fn get_app(&self, app_id: AppId_t, xml_game: &XmlGame) -> Result<AppModel, Box<dyn std::error::Error>> {
        let app_name = self.steam_apps_001.get_app_data(&app_id, &SteamApps001AppDataKeys::Name.as_string())?;
        let developer = self.steam_apps_001.get_app_data(&app_id, &SteamApps001AppDataKeys::Developer.as_string()).unwrap_or("Unknown".to_string());
        let metacritic_score: Option<u8> = self.steam_apps_001
            .get_app_data(&app_id, &SteamApps001AppDataKeys::MetacriticScore.as_string())
            .ok()
            .and_then(|s| s.parse().ok());
        let image_url = self.get_app_image_url(&app_id);

        Ok(AppModel {
            app_id,
            app_name,
            image_url,
            app_type: if xml_game.app_type.as_ref().is_none() { AppModelType::App } else { AppModelType::from_str(&xml_game.app_type.as_ref().unwrap())? },
            developer,
            metacritic_score,
        })
    }

    pub fn get_owned_apps(&self) -> Result<Vec<AppModel>, Box<dyn std::error::Error>> {
        let xml_games = self.get_xml_games()?;

        // IClientUserStats::GetNumAchievedAchievements( 291550, ) = 0,
        // IClientUserStats::GetNumAchievements( 291550, ) = 65

        let mut models = vec![];
        for xml_game in xml_games.games {
            let app_id: AppId_t = xml_game.app_id;

            if self.steam_apps.is_subscribed_app(app_id).unwrap_or(false) == false {
                continue;
            }

            let app= self.get_app(app_id, &xml_game)?;
            models.push(app)
        }

        Ok(models)
    }
}
