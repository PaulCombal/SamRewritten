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
use crate::steam_client::client_user_stats_map_wrapper::ClientUserStatsMap;
use crate::steam_client::client_user_wrapper::ClientUser;
use crate::steam_client::steam_apps_001_wrapper::{SteamApps001, SteamApps001AppDataKeys};
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steamworks_types::{AppId_t, EResult};
use crate::utils::app_paths::get_app_cache_dir;
use crate::utils::ipc_types::SamError;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};

const APP_LIST_URLS: [&str; 2] = [
    "https://raw.githubusercontent.com/PaulCombal/steam-app-list/list/games.xml",
    "https://gib.me/sam/games.xml",
];

pub struct AppLister<'a> {
    app_list_urls: Vec<String>,
    app_list_local: PathBuf,
    current_language: String,
    steam_apps_001: &'a SteamApps001,
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
    pub achievement_count: Option<u32>,
    pub unlocked_achievement_count: Option<u32>,
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

// Error pages and empty bodies parse without errors. This function ensures we find apps.
fn has_games<R: BufRead>(input: R) -> bool {
    let mut count = 0;
    for_each_xml_game(&mut Reader::from_reader(input), |_, _| count += 1).is_ok() && count > 0
}

fn download_app_list(urls: &[String]) -> Result<String, SamError> {
    // The default 30s total timeout is too short on slow links (#93).
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| {
            eprintln!("[ORCHESTRATOR] Failed to build HTTP client: {}", e);
            SamError::AppListRetrievalFailed
        })?;

    for url in urls {
        dev_println!("ORCH", "Downloading app list from:  {}", url);

        let text = client
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.text());

        match text {
            Ok(text) if has_games(text.as_bytes()) => return Ok(text),
            Ok(_) => eprintln!("[ORCHESTRATOR] {url} did not return an app list"),
            Err(e) => eprintln!("[ORCHESTRATOR] Failed to download app list from {url}: {e}"),
        }
    }

    Err(SamError::AppListRetrievalFailed)
}

fn ensure_cached_app_list(
    path: &Path,
    download: impl FnOnce() -> Result<String, SamError>,
) -> Result<(), SamError> {
    let usable = File::open(path).map(BufReader::new).is_ok_and(has_games);
    let fresh = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|modified| modified >= SystemTime::now() - Duration::from_hours(7 * 24));

    if usable && fresh {
        dev_println!("ORCH", "Loading app list from local location");
        return Ok(());
    }

    let app_list_str = match download() {
        Ok(app_list_str) => app_list_str,
        // Left untouched, so its age triggers a retry on the next launch.
        Err(_) if usable => {
            eprintln!("[ORCHESTRATOR] Using the outdated cached app list instead");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    dev_println!(
        "ORCH",
        "App list downloaded. Saving in:  {}",
        path.display()
    );
    fs::write(path, &app_list_str).map_err(|e| {
        eprintln!("[ORCHESTRATOR] Failed to save app list: {}", e);
        SamError::AppListRetrievalFailed
    })
}

impl<'a> AppLister<'a> {
    pub fn new(steam_apps_001: &'a SteamApps001, steam_apps: &'a SteamApps) -> Self {
        let cache_dir = get_app_cache_dir();
        let app_list_urls = match std::env::var("SAM_APP_LIST_URL") {
            Ok(url) => vec![url],
            Err(_) => APP_LIST_URLS.iter().map(|url| url.to_string()).collect(),
        };
        let app_list_local = std::env::var_os("APP_LIST_LOCAL")
            .map(PathBuf::from)
            .unwrap_or(PathBuf::from("apps.xml"));
        let current_language = steam_apps.get_current_game_language();

        AppLister {
            app_list_urls,
            app_list_local: cache_dir.join(app_list_local),
            current_language,
            steam_apps_001,
        }
    }

    fn ensure_local_app_list(&self) -> Result<(), SamError> {
        ensure_cached_app_list(&self.app_list_local, || {
            download_app_list(&self.app_list_urls)
        })
    }

    fn get_app_image_url(&self, app_id: &AppId_t) -> Option<String> {
        let asset = |key: String| {
            self.steam_apps_001
                .get_app_data(app_id, &key)
                .ok()
                .filter(|value| !value.is_empty())
        };

        let fallback = [self.current_language.as_str(), "english"];
        let languages = if self.current_language == "english" {
            &fallback[..1]
        } else {
            &fallback[..]
        };

        // The same 460x215 artwork as a local hit; small_capsule is a
        // different, logo-style design.
        for language in languages {
            if let Some(candidate) =
                asset(SteamApps001AppDataKeys::HeaderImage(language).as_string())
            {
                return Some(format!(
                    "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
                ));
            }
        }

        for language in languages {
            if let Some(candidate) =
                asset(SteamApps001AppDataKeys::SmallCapsule(language).as_string())
            {
                return Some(format!(
                    "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
                ));
            }
        }

        if let Some(candidate) = asset(SteamApps001AppDataKeys::Logo.as_string()) {
            return Some(format!(
                "https://cdn.steamstatic.com/steamcommunity/public/images/apps/{app_id}/{candidate}.jpg"
            ));
        }

        dev_println!("ORCH", "Failed to find image for app {}", app_id);

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
            achievement_count: None,
            unlocked_achievement_count: None,
        })
    }

    pub fn get_owned_apps(
        &self,
        client_user: &ClientUser,
        stats_map: Option<&ClientUserStatsMap>,
    ) -> Result<Vec<AppModel>, SamError> {
        self.ensure_local_app_list()?;

        let owned_set: HashSet<AppId_t> = client_user.get_subscribed_apps().into_iter().collect();

        let file =
            File::open(&self.app_list_local).map_err(|_| SamError::AppListRetrievalFailed)?;
        let mut reader = Reader::from_reader(BufReader::new(file));

        let mut models = Vec::new();
        for_each_xml_game(&mut reader, |app_id, app_type| {
            if !owned_set.contains(&app_id) {
                return;
            }
            match self.build_app_model(app_id, app_type) {
                Ok(app) => models.push(app),
                Err(e) => {
                    dev_println!("ORCH", "Skipping app {app_id}: {e}");
                }
            }
        })?;

        if let Some(map) = stats_map {
            self.populate_achievement_counts(map, &mut models);
        }

        Ok(models)
    }

    fn populate_achievement_counts(&self, map: &ClientUserStatsMap, models: &mut [AppModel]) {
        let app_ids: Vec<AppId_t> = models.iter().map(|m| m.app_id).collect();
        let counts = fetch_achievement_counts(map, &app_ids);
        let by_id: std::collections::HashMap<AppId_t, (u32, u32)> = counts
            .into_iter()
            .map(|(id, total, unlocked)| (id, (total, unlocked)))
            .collect();
        for app in models.iter_mut() {
            if let Some(&(total, unlocked)) = by_id.get(&app.app_id) {
                app.achievement_count = Some(total);
                app.unlocked_achievement_count = Some(unlocked);
            }
        }
    }
}

/// Chunk size shared by the backend pump loop and the GUI achievement loader.
pub const ACHIEVEMENT_COUNT_CHUNK_SIZE: usize = 8;

/// Apps whose schema fails to load within the per-chunk window are omitted.
/// Reads counts as the callback lands: Steam unloads stats nobody looks at.
fn take_answers(
    map: &ClientUserStatsMap,
    answered: &mut HashMap<AppId_t, Option<(u32, u32)>>,
) -> Vec<AppId_t> {
    let mut arrived = Vec::new();
    for (app_id, result) in map.drain_user_stats_callbacks() {
        let counts = if result == EResult::k_EResultOK as i32 {
            let total = map.get_num_achievements(app_id).unwrap_or(0);
            let unlocked = if total > 0 {
                map.get_num_achieved_achievements(app_id).unwrap_or(0)
            } else {
                0
            };
            Some((total, unlocked))
        } else {
            eprintln!("[APP LISTER] Stats for {app_id} came back with result {result}");
            None
        };
        answered.insert(app_id, counts);
        arrived.push(app_id);
    }
    arrived
}

pub fn fetch_achievement_counts(
    map: &ClientUserStatsMap,
    app_ids: &[AppId_t],
) -> Vec<(AppId_t, u32, u32)> {
    const PER_CHUNK_HARD_CAP: Duration = Duration::from_secs(45);
    const NO_PROGRESS_CAP: Duration = Duration::from_secs(3);
    const POLL_INTERVAL: Duration = Duration::from_millis(20);

    map.drain_user_stats_callbacks();

    let mut answered: HashMap<AppId_t, Option<(u32, u32)>> = HashMap::with_capacity(app_ids.len());

    for chunk in app_ids.chunks(ACHIEVEMENT_COUNT_CHUNK_SIZE) {
        let mut pending: HashSet<AppId_t> = HashSet::with_capacity(chunk.len());
        for &app_id in chunk.iter() {
            if answered.contains_key(&app_id) {
                continue;
            }
            if map.request_current_stats(app_id).unwrap_or(false) {
                pending.insert(app_id);
            } else {
                dev_println!("APP LISTER", "Steam refused a stats request for {app_id}");
            }
        }

        let hard_deadline = Instant::now() + PER_CHUNK_HARD_CAP;
        let mut last_progress = Instant::now();
        while !pending.is_empty() && Instant::now() < hard_deadline {
            map.run_engine_frame();
            for app_id in take_answers(map, &mut answered) {
                if pending.remove(&app_id) {
                    last_progress = Instant::now();
                }
            }
            if pending.is_empty() || last_progress.elapsed() >= NO_PROGRESS_CAP {
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    map.run_engine_frame();
    take_answers(map, &mut answered);

    let mut out: Vec<(AppId_t, u32, u32)> = Vec::with_capacity(app_ids.len());
    let mut seen: HashSet<AppId_t> = HashSet::with_capacity(app_ids.len());
    for &app_id in app_ids {
        if !seen.insert(app_id) {
            continue;
        }
        if let Some(Some((total, unlocked))) = answered.get(&app_id) {
            out.push((app_id, *total, *unlocked));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const OLD_LIST: &str = "<games><game>480</game></games>";
    const NEW_LIST: &str = "<games><game>440</game></games>";

    struct Cache(PathBuf);

    impl Cache {
        fn new(name: &str, contents: Option<&str>, stale: bool) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "samrewritten-app-list-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir(&dir).unwrap();
            let path = dir.join("apps.xml");
            if let Some(contents) = contents {
                fs::write(&path, contents).unwrap();
                if stale {
                    File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_modified(SystemTime::now() - Duration::from_hours(8 * 24))
                        .unwrap();
                }
            }
            Self(path)
        }

        fn contents(&self) -> String {
            fs::read_to_string(&self.0).unwrap()
        }
    }

    impl Drop for Cache {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    fn serve_once(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/games.xml", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 4096]);
            stream.write_all(response.as_bytes()).unwrap();
        });
        url
    }

    #[test]
    fn a_fresh_cache_is_not_downloaded_again() {
        let cache = Cache::new("fresh", Some(OLD_LIST), false);
        ensure_cached_app_list(&cache.0, || panic!("unexpected download")).unwrap();
        assert_eq!(cache.contents(), OLD_LIST);
    }

    #[test]
    fn a_stale_cache_is_replaced() {
        let cache = Cache::new("stale", Some(OLD_LIST), true);
        ensure_cached_app_list(&cache.0, || Ok(NEW_LIST.into())).unwrap();
        assert_eq!(cache.contents(), NEW_LIST);
    }

    #[test]
    fn a_failed_refresh_keeps_the_old_list_and_its_age() {
        let cache = Cache::new("failed-refresh", Some(OLD_LIST), true);
        let modified = fs::metadata(&cache.0).unwrap().modified().unwrap();
        ensure_cached_app_list(&cache.0, || Err(SamError::AppListRetrievalFailed)).unwrap();
        assert_eq!(cache.contents(), OLD_LIST);
        assert_eq!(
            fs::metadata(&cache.0).unwrap().modified().unwrap(),
            modified
        );
    }

    #[test]
    fn a_failed_first_download_is_an_error() {
        let cache = Cache::new("first-launch", None, false);
        assert!(
            ensure_cached_app_list(&cache.0, || Err(SamError::AppListRetrievalFailed)).is_err()
        );
    }

    #[test]
    fn an_unusable_cache_is_downloaded_again_even_when_fresh() {
        let cache = Cache::new("unusable", Some("<html>Bad Gateway</html>"), false);
        ensure_cached_app_list(&cache.0, || Ok(NEW_LIST.into())).unwrap();
        assert_eq!(cache.contents(), NEW_LIST);
    }

    #[test]
    fn only_a_list_with_games_counts() {
        assert!(has_games(OLD_LIST.as_bytes()));
        for not_a_list in [
            "",
            "Bad Gateway",
            "<html><body>Wi-Fi login</body></html>",
            "<games></games>",
        ] {
            assert!(!has_games(not_a_list.as_bytes()), "{not_a_list:?}");
        }
    }

    #[test]
    fn a_failing_source_falls_through_to_the_next() {
        let urls = [
            serve_once(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ),
            serve_once(
                "HTTP/1.1 200 OK\r\nContent-Length: 18\r\nConnection: close\r\n\r\n<html>login</html>",
            ),
            serve_once(
                "HTTP/1.1 200 OK\r\nContent-Length: 31\r\nConnection: close\r\n\r\n<games><game>440</game></games>",
            ),
        ];
        assert_eq!(download_app_list(&urls).unwrap(), NEW_LIST);
    }
}
