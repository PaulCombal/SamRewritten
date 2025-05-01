use crate::steam_client::create_client::create_steam_client;
use crate::steam_client::steam_apps_001_wrapper::SteamApps001;
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steam_client_wrapper::SteamClient;
use crate::steam_client::steam_user_stats_wrapper::SteamUserStats;
use crate::steam_client::steam_utils_wrapper::SteamUtils;
use crate::steam_client::types::{HSteamPipe, HSteamUser};

pub struct ConnectedSteam<'a> {
    pipe: HSteamPipe,
    user: HSteamUser,
    pub client: SteamClient<'a>,
    pub apps_001: SteamApps001,
    pub apps: SteamApps,
    pub utils: SteamUtils,
    pub user_stats: SteamUserStats,
}

impl<'a> ConnectedSteam<'a> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = create_steam_client()?;
        let pipe = client.create_steam_pipe()?;
        let user = client.connect_to_global_user(pipe)?;
        let apps = client.get_isteam_apps(user, pipe)?;
        let apps_001 = client.get_isteam_apps_001(user, pipe)?;
        let utils = client.get_isteam_utils(pipe)?;
        let user_stats = client.get_isteam_user_stats(user, pipe)?;

        Ok(ConnectedSteam {
            pipe,
            user,
            client,
            apps,
            apps_001,
            utils,
            user_stats,
        })
    }

    pub fn shutdown(&self) {
        self.client.release_user(self.pipe, self.user);
        self.client.release_steam_pipe(self.pipe).expect("Failed to release steam pipe");
        let _ = self.client.shutdown_if_app_pipes_closed();
    }
}