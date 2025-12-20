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

use crate::steam_client::create_client::create_steam_client;
use crate::steam_client::steam_apps_001_wrapper::SteamApps001;
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steam_client_wrapper::SteamClient;
use crate::steam_client::steam_user_stats_wrapper::SteamUserStats;
use crate::steam_client::steam_user_wrapper::SteamUser;
use crate::steam_client::steam_utils_wrapper::SteamUtils;
use crate::steam_client::steamworks_types::{HSteamPipe, HSteamUser};

pub struct ConnectedSteam {
    h_pipe: HSteamPipe,
    h_user: HSteamUser,
    pub client: SteamClient,
    pub apps_001: SteamApps001,
    pub apps: SteamApps,
    pub user_stats: SteamUserStats,
    pub utils: SteamUtils,
    pub user: SteamUser,
}

impl<'a> ConnectedSteam {
    pub fn new(silent: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let client = create_steam_client(silent)?;
        let h_pipe = client.create_steam_pipe()?;
        let h_user = client.connect_to_global_user(h_pipe)?;
        let apps = client.get_isteam_apps(h_user, h_pipe)?;
        let utils = client.get_isteam_utils(h_pipe)?;
        let apps_001 = client.get_isteam_apps_001(h_user, h_pipe)?;
        let user_stats = client.get_isteam_user_stats(h_user, h_pipe)?;
        let user = client.get_isteam_user(h_user, h_pipe)?;

        Ok(ConnectedSteam {
            h_pipe,
            h_user,
            client,
            apps,
            apps_001,
            user_stats,
            utils,
            user,
        })
    }

    pub fn shutdown(&self) {
        self.client.release_user(self.h_pipe, self.h_user);
        self.client
            .release_steam_pipe(self.h_pipe)
            .expect("Failed to release steam pipe");
        let _ = self.client.shutdown_if_app_pipes_closed();
    }
}

impl Drop for ConnectedSteam {
    fn drop(&mut self) {
        self.shutdown();
    }
}
