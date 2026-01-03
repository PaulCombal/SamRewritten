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

#[cfg(test)]
mod tests {
    use crate::backend::app_manager::AppManager;
    use crate::backend::connected_steam::ConnectedSteam;
    use crate::backend::key_value::KeyValue;
    use crate::steam_client::steam_apps_001_wrapper::SteamApps001AppDataKeys;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn get_achievements_with_callback() {
        let mut app_manager =
            AppManager::new_connected(206690).expect("Failed to create app manager");
        let achievements = app_manager
            .get_achievements()
            .expect("Failed to get achievements");
        println!("{achievements:?}")
    }

    #[test]
    fn get_stats_no_message() {
        let mut app_manager = AppManager::new_connected(480).expect("Failed to create app manager");
        let stats = app_manager.get_statistics().expect("Failed to get stats");
        println!("{stats:?}")
    }

    #[test]
    fn reset_stats_no_message() {
        let app_manager = AppManager::new_connected(480).expect("Failed to create app manager");
        let success = app_manager
            .reset_all_stats(true)
            .expect("Failed to get stats");
        println!("Success: {success:?}")
    }

    #[test]
    fn brute_force_app001_keys() {
        // Find others on your own with the Steam command app_info_print

        let connected_steam = ConnectedSteam::new(true).expect("Failed to create connected steam");
        let try_force = |key: &str| {
            let null_terminated_key = format!("{key}\0");
            println!(
                "{key}:\t {}",
                connected_steam
                    .apps_001
                    .get_app_data(&220, &null_terminated_key)
                    .unwrap_or("Failure".to_string())
            );
        };

        try_force(&SteamApps001AppDataKeys::Name.as_string());
        try_force(&SteamApps001AppDataKeys::Logo.as_string());
        try_force(&SteamApps001AppDataKeys::SmallCapsule("english").as_string());
        try_force("subscribed");

        try_force("metascore");
        try_force("metascore/score");
        try_force("metascorescore");
        try_force("metascorerating");
        try_force("metascore/rating");
        try_force("metascore_rating");
        try_force("metascore_rating");

        try_force("metacritic");
        try_force("metacritic/score");
        try_force("metacritic/url");
        try_force("metacriticurl/english");
        try_force("metacritic/url/english");
        try_force("metacriticscore");
        try_force("metacritic_score");
        try_force("metacriticrating");
        try_force("metacritic/rating");
        try_force("metacritic_rating");
        try_force("metacritic_rating");

        try_force("developer");
        try_force("developer/english");
        try_force("extended/developer");
        try_force("state");
        try_force("homepage");
        try_force("clienticon");
    }

    #[test]
    fn keyval() {
        #[cfg(target_os = "linux")]
        let home = env::var("HOME").expect("Failed to get home directory");
        #[cfg(target_os = "linux")]
        let bin_file = PathBuf::from(
            home + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_730.bin",
        );
        #[cfg(target_os = "windows")]
        let program_files =
            env::var("ProgramFiles(x86)").expect("Failed to get Program Files directory");
        #[cfg(target_os = "windows")]
        let bin_file =
            PathBuf::from(program_files + "\\Steam\\appcache\\stats\\UserGameStatsSchema_480.bin");

        let kv = KeyValue::load_as_binary(bin_file).expect("Failed to load key value");
        println!("{kv:?}");
    }
}
