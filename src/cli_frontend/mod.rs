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

use crate::backend::app_lister::AppLister;
use crate::backend::app_manager::AppManager;
use crate::backend::connected_steam::ConnectedSteam;
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[clap(
    author,
    version,
    long_about = "Steam Achievements Manager Rewritten\nLicensed under GNU GPLv3, Copyright (C) 2026"
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
    UnlockAll {
        app_id: u32,
    },
    Lock {
        app_id: u32,
        #[command(flatten)]
        ids: Ids,
    },
    LockAll {
        app_id: u32,
    },
    Idle {
        app_id: u32,
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

            let achievements = match manager.get_achievements(true) {
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
                Ok(_) => {}
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

        Command::UnlockAll { app_id } => {
            let mut manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            match manager.unlock_all_achievements() {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to unlock all achievements: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            }

            let status = json!({"success": true});
            println!("{}", status);
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
                Ok(_) => {}
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

        Command::Idle { app_id } => {
            let _manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            if let Err(e) = install_interrupt_handler() {
                eprintln!("Failed to install interrupt handler: {}", e);
                return std::process::ExitCode::FAILURE;
            }

            eprintln!("Idling app {}. Press Ctrl+C to stop.", app_id);
            while !INTERRUPTED.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            eprintln!("Stopping idle for app {}...", app_id);
            // _manager drops here → ConnectedSteam::drop releases the pipe cleanly.
        }

        Command::LockAll { app_id } => {
            let manager = match AppManager::new_connected(app_id) {
                Ok(manager) => manager,
                Err(e) => {
                    eprintln!("Failed to connect to Steam: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            };

            match manager.reset_all_stats(true) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to reset all achievements: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            }

            let status = json!({"success": true});
            println!("{}", status);
        }
    }

    std::process::ExitCode::SUCCESS
}

#[cfg(unix)]
fn install_interrupt_handler() -> Result<(), &'static str> {
    use std::os::raw::c_int;

    const SIGINT: c_int = 2;
    const SIGTERM: c_int = 15;
    type SigHandler = extern "C" fn(c_int);

    unsafe extern "C" {
        fn signal(signum: c_int, handler: SigHandler) -> SigHandler;
    }

    extern "C" fn on_signal(_: c_int) {
        INTERRUPTED.store(true, Ordering::SeqCst);
    }

    unsafe {
        signal(SIGINT, on_signal);
        signal(SIGTERM, on_signal);
    }
    Ok(())
}

#[cfg(windows)]
fn install_interrupt_handler() -> Result<(), &'static str> {
    type Bool = i32;
    type Dword = u32;
    type PhandlerRoutine = unsafe extern "system" fn(Dword) -> Bool;

    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<PhandlerRoutine>, add: Bool) -> Bool;
    }

    unsafe extern "system" fn on_ctrl(ctrl_type: Dword) -> Bool {
        // CTRL_C_EVENT, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT
        if ctrl_type <= 2 {
            INTERRUPTED.store(true, Ordering::SeqCst);
            1
        } else {
            0
        }
    }

    let ok = unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
    if ok == 0 {
        Err("SetConsoleCtrlHandler returned FALSE")
    } else {
        Ok(())
    }
}
