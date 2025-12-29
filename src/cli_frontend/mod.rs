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

use crate::backend::app_lister::AppLister;
use crate::backend::app_manager::AppManager;
use crate::backend::connected_steam::ConnectedSteam;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(
    author,
    version,
    long_about = "Steam Achievements Manager Rewritten\nLicensed under GNU GPLv3, Copyright (c) 2025"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    ListAchievements {
        app_id: u32,
    },
    ListStatistics {
        app_id: u32,
    },
    ListApps,
    Unlock {
        app_id: u32,
        #[command(flatten)]
        ids: Ids,
    },
    Lock {
        app_id: u32,
        #[command(flatten)]
        ids: Ids,
    },
}

#[derive(Args)]
struct Ids {
    #[arg(required = true)]
    ids: Vec<String>,
}

/// Decorated main function.
pub fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::ListAchievements { app_id } => {
            let mut manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            let achievements = match manager.get_achievements() {
                Ok(achievements) => achievements,
                Err(e) => {
                    eprintln!("Failed to get achievements: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            match serde_json::to_string_pretty(&achievements) {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("Failed to serialize achievements: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };
        }
        Command::ListStatistics { app_id } => {
            let mut manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            let statistics = match manager.get_statistics() {
                Ok(statistics) => statistics,
                Err(e) => {
                    eprintln!("Failed to get statistics: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            match serde_json::to_string_pretty(&statistics) {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("Failed to serialize statistics: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };
        }
        Command::ListApps => {
            let connected_steam = match ConnectedSteam::new(false) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            let apps_001 = &connected_steam.apps_001;
            let apps = &connected_steam.apps;
            let app_lister = AppLister::new(apps_001, apps);

            match app_lister.get_owned_apps() {
                Ok(apps) => {
                    match serde_json::to_string_pretty(&apps) {
                        Ok(output) => println!("{}", output),
                        Err(e) => {
                            eprintln!("Failed to serialize apps: {}", e);
                            return std::process::ExitCode::FAILURE;
                        }
                    };
                }
                Err(e) => {
                    eprintln!("Failed to get owned apps: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };
        }

        Command::Unlock { app_id, ids } => {
            let manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            #[derive(serde::Serialize)]
            struct AchievedResult {
                id: String,
                success: bool,
            }

            let mut results: Vec<AchievedResult> = vec![];

            for id in ids.ids {
                match manager.set_achievement(&id, true, false) {
                    Ok(_) => results.push(AchievedResult { id, success: true }),
                    Err(e) => {
                        println!("Failed to unlock achievement {}: {}", id, e);
                        results.push(AchievedResult { id, success: false });
                    }
                };
            }

            match manager.store_stats_and_achievements() {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to store stats and achievements: {e:?}");
                    return std::process::ExitCode::FAILURE;
                }
            };

            match serde_json::to_string_pretty(&results) {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("Failed to serialize achievements unlock result: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };
        }

        Command::Lock { app_id, ids } => {
            let manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            #[derive(serde::Serialize)]
            struct AchievedResult {
                id: String,
                success: bool,
            }

            let mut results: Vec<AchievedResult> = vec![];

            for id in ids.ids {
                match manager.set_achievement(&id, false, false) {
                    Ok(_) => results.push(AchievedResult { id, success: true }),
                    Err(e) => {
                        println!("Failed to lock achievement {}: {}", id, e);
                        results.push(AchievedResult { id, success: false });
                    }
                };
            }

            match manager.store_stats_and_achievements() {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to store stats and achievements: {e:?}");
                    return std::process::ExitCode::FAILURE;
                }
            };

            match serde_json::to_string_pretty(&results) {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("Failed to serialize achievements lock result: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };
        }
    }

    std::process::ExitCode::SUCCESS
}
