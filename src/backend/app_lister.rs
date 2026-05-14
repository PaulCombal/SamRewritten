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

use crate::dev_println;
use crate::steam_client::steam_apps_001_wrapper::{SteamApps001, SteamApps001AppDataKeys};
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::app_paths::get_app_cache_dir;
use crate::utils::ipc_types::SamError;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

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
    pub playtime_minutes: Option<u32>,
    pub last_played: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum AppModelType {
    App,
    Mod,
    Demo,
    Junk,
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

/// Walks `<game type="...">N</game>` elements from `reader`, calling `visit`
/// for each. Avoids loading the full ~200k-entry list into memory.
fn for_each_xml_game<R: BufRead>(
    reader: &mut Reader<R>,
    mut visit: impl FnMut(u32, Option<&str>),
) -> Result<(), SamError> {
    let mut buf = Vec::with_capacity(256);
    let mut in_game = false;
    let mut game_type: Option<String> = None;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"game" => {
                in_game = true;
                game_type = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .find(|a| a.key.as_ref() == b"type")
                    .and_then(|a| {
                        std::str::from_utf8(a.value.as_ref())
                            .ok()
                            .map(|s| s.to_owned())
                    });
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"game" => {
                in_game = false;
                game_type = None;
            }
            Ok(Event::Text(t)) if in_game => {
                if let Ok(text) = t.decode()
                    && let Ok(app_id) = text.trim().parse::<u32>()
                {
                    visit(app_id, game_type.as_deref());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("[ORCHESTRATOR] XML parse error: {e}");
                return Err(SamError::AppListRetrievalFailed);
            }
            _ => {}
        }
    }
    Ok(())
}
impl<'a> AppLister<'a> {
    pub fn new(steam_apps_001: &'a SteamApps001, steam_apps: &'a SteamApps) -> Self {
        let cache_dir = get_app_cache_dir();
        let app_list_url = std::env::var("SAM_APP_LIST_URL")
            .unwrap_or(String::from("https://gib.me/sam/games.xml"));
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

    fn download_app_list_str(&self) -> Result<String, SamError> {
        dev_println!(
            "[ORCHESTRATOR] Downloading app list from:  {}",
            &self.app_list_url
        );

        let response = reqwest::blocking::get(&self.app_list_url)
            .map_err(|e| {
                eprintln!("[ORCHESTRATOR] Failed to download app list: {}", e);
                SamError::AppListRetrievalFailed
            })?
            .text()
            .map_err(|e| {
                eprintln!("[ORCHESTRATOR] Failed to decode text from app list: {}", e);
                SamError::AppListRetrievalFailed
            })?;

        Ok(response)
    }

    /// Refresh the local cached file if missing or older than a week. Returns
    /// once the file at `self.app_list_local` is up to date.
    fn ensure_local_app_list(&self) -> Result<(), SamError> {
        let should_update = match fs::metadata(&self.app_list_local) {
            Ok(metadata) => {
                let last_update = metadata
                    .modified()
                    .map_err(|_| SamError::AppListRetrievalFailed)?;
                let one_week_ago = SystemTime::now() - Duration::from_hours(7 * 24);
                last_update < one_week_ago
            }
            Err(_) => true,
        };

        if should_update {
            let app_list_str = self.download_app_list_str()?;
            dev_println!(
                "[ORCHESTRATOR] App list downloaded. Saving in:  {}",
                &self.app_list_local
            );
            fs::write(&self.app_list_local, &app_list_str).map_err(|e| {
                eprintln!("[ORCHESTRATOR] Failed to save app list: {}", e);
                SamError::AppListRetrievalFailed
            })?;
        } else {
            dev_println!("[ORCHESTRATOR] Loading app list from local location");
        }
        Ok(())
    }

    fn get_app_image_url(&self, app_id: &AppId_t) -> Option<String> {
        let candidate = self
            .steam_apps_001
            .get_app_data(
                app_id,
                &SteamApps001AppDataKeys::SmallCapsule(&self.current_language).as_string(),
            )
            .unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!(
                "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
            ));
        }

        if self.current_language != "english" {
            let candidate = self
                .steam_apps_001
                .get_app_data(
                    app_id,
                    &SteamApps001AppDataKeys::SmallCapsule("english").as_string(),
                )
                .unwrap_or("".to_owned());
            if !candidate.is_empty() {
                return Some(format!(
                    "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
                ));
            }
        }

        let candidate = self
            .steam_apps_001
            .get_app_data(app_id, &SteamApps001AppDataKeys::Logo.as_string())
            .unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!(
                "https://cdn.steamstatic.com/steamcommunity/public/images/apps/{app_id}/{candidate}.jpg"
            ));
        }

        dev_println!("[ORCHESTRATOR] Failed to find image for app {}", app_id);

        None
    }

    fn build_app_model(
        &self,
        app_id: AppId_t,
        app_type: Option<&str>,
    ) -> Result<AppModel, SamError> {
        let app_name = self
            .steam_apps_001
            .get_app_data(&app_id, &SteamApps001AppDataKeys::Name.as_string())
            .map_err(|_| SamError::AppListRetrievalFailed)?;
        let developer = self
            .steam_apps_001
            .get_app_data(&app_id, &SteamApps001AppDataKeys::Developer.as_string())
            .unwrap_or("Unknown".to_string());
        let metacritic_score: Option<u8> = self
            .steam_apps_001
            .get_app_data(
                &app_id,
                &SteamApps001AppDataKeys::MetacriticScore.as_string(),
            )
            .ok()
            .and_then(|s| s.parse().ok());
        let image_url = self.get_app_image_url(&app_id);

        let app_type = match app_type {
            None => AppModelType::App,
            Some(s) => AppModelType::from_str(s).map_err(|_| SamError::AppListRetrievalFailed)?,
        };

        Ok(AppModel {
            app_id,
            app_name,
            image_url,
            app_type,
            developer,
            metacritic_score,
            playtime_minutes: None,
            last_played: None,
        })
    }

    pub fn get_owned_apps(&self) -> Result<Vec<AppModel>, SamError> {
        self.ensure_local_app_list()?;

        let file =
            File::open(&self.app_list_local).map_err(|_| SamError::AppListRetrievalFailed)?;
        let mut reader = Reader::from_reader(BufReader::new(file));

        let mut models = Vec::new();
        for_each_xml_game(&mut reader, |app_id, app_type| {
            if !self.steam_apps.is_subscribed_app(app_id).unwrap_or(false) {
                return;
            }
            match self.build_app_model(app_id, app_type) {
                Ok(app) => models.push(app),
                Err(e) => {
                    dev_println!("[ORCHESTRATOR] Skipping app {app_id}: {e}");
                }
            }
        })?;

        Ok(models)
    }
}
