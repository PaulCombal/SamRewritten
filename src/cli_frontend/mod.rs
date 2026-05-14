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
use crate::backend::progress_io::{
    MAX_CONCURRENT_APPS, parse_response_bytes, run_command_on_apps_concurrent,
};
use crate::utils::export_file::{ExportFile, FORMAT_VERSION, iso8601_utc_now};
use crate::utils::ipc_types::{AppExport, ImportSummary, SteamCommand};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
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
    /// Export achievements and stats for one or more apps to stdout as JSON.
    Export {
        #[arg(required = true)]
        app_ids: Vec<u32>,
    },
    /// Import achievements and stats from a JSON file produced by `export`
    /// (or by the GUI). Protected fields are skipped. Prints a JSON summary.
    Import {
        file: PathBuf,
        /// Only import the app with this ID (skip the rest).
        #[arg(long)]
        app_id: Option<u32>,
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

        Command::Export { app_ids } => {
            let items: Vec<(u32, SteamCommand)> = app_ids
                .iter()
                .map(|id| (*id, SteamCommand::ExportAppProgress(*id)))
                .collect();
            let results = run_command_on_apps_concurrent(items, MAX_CONCURRENT_APPS, None);

            let mut by_id: std::collections::HashMap<u32, Result<AppExport, String>> = results
                .into_iter()
                .map(|(id, raw)| {
                    let result = raw
                        .and_then(|bytes| parse_response_bytes::<AppExport>(&bytes))
                        .map_err(|e| e.to_string());
                    (id, result)
                })
                .collect();

            let mut apps: Vec<AppExport> = Vec::new();
            let mut failed = false;
            for app_id in app_ids {
                match by_id.remove(&app_id) {
                    Some(Ok(export)) => apps.push(export),
                    Some(Err(e)) => {
                        eprintln!("App {app_id}: {e}");
                        failed = true;
                    }
                    None => {
                        eprintln!("App {app_id}: missing from batch result");
                        failed = true;
                    }
                }
            }

            let file = ExportFile {
                format_version: FORMAT_VERSION,
                exported_at: iso8601_utc_now(),
                apps,
            };

            match serde_json::to_string_pretty(&file) {
                Ok(out) => println!("{}", out),
                Err(e) => {
                    eprintln!("Failed to serialize export: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            }

            if failed {
                return std::process::ExitCode::FAILURE;
            }
        }

        Command::Import { file, app_id } => {
            let contents = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read {}: {}", file.display(), e);
                    return std::process::ExitCode::FAILURE;
                }
            };
            let parsed: ExportFile = match serde_json::from_str(&contents) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse {}: {}", file.display(), e);
                    return std::process::ExitCode::FAILURE;
                }
            };
            if parsed.format_version != FORMAT_VERSION {
                eprintln!(
                    "Unsupported format version: {} (this build expects {})",
                    parsed.format_version, FORMAT_VERSION
                );
                return std::process::ExitCode::FAILURE;
            }

            #[derive(Serialize)]
            struct AppResult {
                app_id: u32,
                #[serde(flatten)]
                summary: ImportSummary,
                #[serde(skip_serializing_if = "Option::is_none")]
                error: Option<String>,
            }

            let apps: Vec<AppExport> = parsed
                .apps
                .into_iter()
                .filter(|a| app_id.map(|wanted| wanted == a.app_id).unwrap_or(true))
                .collect();

            if apps.is_empty() {
                eprintln!("No matching apps to import.");
                return std::process::ExitCode::FAILURE;
            }

            let app_ids: Vec<u32> = apps.iter().map(|a| a.app_id).collect();
            let items: Vec<(u32, SteamCommand)> = apps
                .into_iter()
                .map(|a| (a.app_id, SteamCommand::ImportAppProgress(a.app_id, a)))
                .collect();
            let raw_results = run_command_on_apps_concurrent(items, MAX_CONCURRENT_APPS, None);

            let mut by_id: std::collections::HashMap<u32, Result<ImportSummary, String>> =
                raw_results
                    .into_iter()
                    .map(|(id, raw)| {
                        let result = raw
                            .and_then(|bytes| parse_response_bytes::<ImportSummary>(&bytes))
                            .map_err(|e| e.to_string());
                        (id, result)
                    })
                    .collect();

            let mut results: Vec<AppResult> = Vec::new();
            let mut any_failure = false;
            for id in app_ids {
                match by_id.remove(&id) {
                    Some(Ok(summary)) => {
                        if !summary.errors.is_empty() {
                            any_failure = true;
                        }
                        results.push(AppResult {
                            app_id: id,
                            summary,
                            error: None,
                        });
                    }
                    Some(Err(e)) => {
                        any_failure = true;
                        results.push(AppResult {
                            app_id: id,
                            summary: ImportSummary::default(),
                            error: Some(e),
                        });
                    }
                    None => {
                        any_failure = true;
                        results.push(AppResult {
                            app_id: id,
                            summary: ImportSummary::default(),
                            error: Some("missing from batch result".to_string()),
                        });
                    }
                }
            }

            match serde_json::to_string_pretty(&results) {
                Ok(out) => println!("{}", out),
                Err(e) => {
                    eprintln!("Failed to serialize import summary: {}", e);
                    return std::process::ExitCode::FAILURE;
                }
            }

            if any_failure {
                return std::process::ExitCode::FAILURE;
            }
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
