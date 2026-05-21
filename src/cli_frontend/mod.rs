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

use crate::backend::orchestrator_client::{
    ExportApps, GetAchievementsAndStats, GetSubscribedAppList, ImportApps, LaunchApp, Request,
    ResetStats, SetAchievement, StoreStatsAndAchievements, UnlockAllAchievements, set_orchestrator,
    shutdown_and_wait,
};
use crate::utils::app_paths::get_executable_path;
use crate::utils::bidir_child::BidirChild;
use crate::utils::export_file::{ExportFile, FORMAT_VERSION, iso8601_utc_now};
use crate::utils::ipc_client::IpcClient;
use crate::utils::ipc_types::{AppExport, ImportSummary, SamError};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, ExitCode};
use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[clap(
    author,
    version,
    about = "Manage Steam achievements and stats from the command line.",
    long_about = "Steam Achievements Manager Rewritten\n\
                  Manage Steam achievements and stats for the apps your account owns: \
                  list, unlock, lock, idle, and import/export progress as JSON.\n\
                  Requires the Steam client to be running and signed in.\n\
                  Licensed under GNU GPLv3, Copyright (C) 2026"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all achievements for an app, with their current unlock status, as JSON.
    ListAchievements {
        /// Steam AppID of the game to query.
        app_id: u32,
    },
    /// List all stats defined for an app, with their current values, as JSON.
    ListStatistics {
        /// Steam AppID of the game to query.
        app_id: u32,
    },
    /// List all apps owned by the logged-in Steam user as JSON.
    ListApps {
        /// Also include per-app achievement counts (total and unlocked).
        /// Slower: requires querying stats for every owned app.
        #[arg(long)]
        with_achievements: bool,
    },
    /// Unlock one or more achievements for an app.
    Unlock {
        /// Steam AppID of the game.
        app_id: u32,
        #[command(flatten)]
        ids: Ids,
    },
    /// Unlock every achievement defined for an app.
    UnlockAll {
        /// Steam AppID of the game.
        app_id: u32,
    },
    /// Lock (re-lock) one or more achievements for an app.
    Lock {
        /// Steam AppID of the game.
        app_id: u32,
        #[command(flatten)]
        ids: Ids,
    },
    /// Reset every achievement and stat for an app to its locked/default state.
    LockAll {
        /// Steam AppID of the game.
        app_id: u32,
    },
    /// Idle an app (appear in-game) until interrupted with Ctrl+C.
    Idle {
        /// Steam AppID of the game to idle.
        app_id: u32,
    },
    /// Export achievements and stats for one or more apps to stdout as JSON.
    Export {
        /// One or more Steam AppIDs to export.
        #[arg(required = true)]
        app_ids: Vec<u32>,
    },
    /// Import achievements and stats from a JSON file produced by `export`
    /// (or by the GUI). Protected fields are skipped. Prints a JSON summary.
    Import {
        /// Path to a JSON file previously produced by `export` or the GUI.
        file: PathBuf,
        /// Only import the app with this ID (skip the rest).
        #[arg(long)]
        app_id: Option<u32>,
    },
}

#[derive(Args)]
struct Ids {
    /// One or more achievement API names to act on.
    #[arg(required = true)]
    ids: Vec<String>,
}

/// The orchestrator owns every Steam connection, so the CLI process itself
/// never loads `steamclient.so`.
fn spawn_orchestrator() -> Result<(), SamError> {
    let child = BidirChild::new(ProcessCommand::new(get_executable_path()).arg("--orchestrator"))?;
    set_orchestrator(IpcClient::new(child));
    Ok(())
}

/// Decorated main function.
pub fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = spawn_orchestrator() {
        eprintln!("Failed to start the backend process: {e}");
        return ExitCode::FAILURE;
    }

    let code = run_command(cli.command);
    shutdown_and_wait();
    code
}

fn run_command(command: Command) -> ExitCode {
    match command {
        Command::ListAchievements { app_id } => {
            let (achievements, _stats) = match (GetAchievementsAndStats {
                app_id,
                launch: true,
            })
            .request()
            {
                Ok(progress) => progress,
                Err(e) => {
                    eprintln!("Failed to get achievements: {e}");
                    return ExitCode::FAILURE;
                }
            };
            print_json(&achievements)
        }

        Command::ListStatistics { app_id } => {
            let (_achievements, statistics) = match (GetAchievementsAndStats {
                app_id,
                launch: true,
            })
            .request()
            {
                Ok(progress) => progress,
                Err(e) => {
                    eprintln!("Failed to get statistics: {e}");
                    return ExitCode::FAILURE;
                }
            };
            print_json(&statistics)
        }

        Command::ListApps { with_achievements } => {
            let apps = match (GetSubscribedAppList {
                include_playtime: false,
                with_achievement_counts: with_achievements,
            })
            .request()
            {
                Ok(apps) => apps,
                Err(e) => {
                    eprintln!("Failed to get owned apps: {e}");
                    return ExitCode::FAILURE;
                }
            };
            print_json(&apps)
        }

        Command::Unlock { app_id, ids } => set_achievements(app_id, ids.ids, true),

        Command::Lock { app_id, ids } => set_achievements(app_id, ids.ids, false),

        Command::UnlockAll { app_id } => match (UnlockAllAchievements { app_id }).request() {
            Ok(_) => {
                println!("{}", json!({"success": true}));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Failed to unlock all achievements: {e}");
                ExitCode::FAILURE
            }
        },

        Command::LockAll { app_id } => match (ResetStats {
            app_id,
            achievements_too: true,
        })
        .request()
        {
            Ok(_) => {
                println!("{}", json!({"success": true}));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Failed to reset all achievements: {e}");
                ExitCode::FAILURE
            }
        },

        Command::Idle { app_id } => {
            if let Err(e) = (LaunchApp { app_id }).request() {
                eprintln!("Failed to connect to Steam: {e}");
                return ExitCode::FAILURE;
            }

            if let Err(e) = install_interrupt_handler() {
                eprintln!("Failed to install interrupt handler: {}", e);
                return ExitCode::FAILURE;
            }

            eprintln!("Idling app {}. Press Ctrl+C to stop.", app_id);
            while !INTERRUPTED.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            eprintln!("Stopping idle for app {}...", app_id);
            // The app-server child is torn down by the orchestrator shutdown in main().
            ExitCode::SUCCESS
        }

        Command::Export { app_ids } => export(app_ids),

        Command::Import { file, app_id } => import(file, app_id),
    }
}

/// Output mirrors the legacy in-process CLI: a JSON array of `{id, success}`.
fn set_achievements(app_id: u32, ids: Vec<String>, unlocked: bool) -> ExitCode {
    #[derive(Serialize)]
    struct AchievedResult {
        id: String,
        success: bool,
    }
    let verb = if unlocked { "unlock" } else { "lock" };

    if let Err(e) = (LaunchApp { app_id }).request() {
        eprintln!("Failed to connect to Steam: {e}");
        return ExitCode::FAILURE;
    }

    let mut results: Vec<AchievedResult> = vec![];
    for id in ids {
        let success = (SetAchievement {
            app_id,
            achievement_id: id.clone(),
            unlocked,
            store: false,
        })
        .request()
        .is_ok();
        if !success {
            println!("Failed to {verb} achievement {id}");
        }
        results.push(AchievedResult { id, success });
    }

    if let Err(e) = (StoreStatsAndAchievements { app_id }).request() {
        eprintln!("Failed to store stats and achievements: {e:?}");
        return ExitCode::FAILURE;
    }

    print_json(&results)
}

fn export(app_ids: Vec<u32>) -> ExitCode {
    let results = match (ExportApps {
        app_ids: app_ids.clone(),
    })
    .request()
    {
        Ok(results) => results,
        Err(e) => {
            eprintln!("Failed to export: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut by_id: std::collections::HashMap<u32, Result<AppExport, SamError>> =
        results.into_iter().collect();

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
            return ExitCode::FAILURE;
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn import(file: PathBuf, app_id: Option<u32>) -> ExitCode {
    let contents = match std::fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read {}: {}", file.display(), e);
            return ExitCode::FAILURE;
        }
    };
    let parsed: ExportFile = match serde_json::from_str(&contents) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", file.display(), e);
            return ExitCode::FAILURE;
        }
    };
    if parsed.format_version != FORMAT_VERSION {
        eprintln!(
            "Unsupported format version: {} (this build expects {})",
            parsed.format_version, FORMAT_VERSION
        );
        return ExitCode::FAILURE;
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
        return ExitCode::FAILURE;
    }

    let app_ids: Vec<u32> = apps.iter().map(|a| a.app_id).collect();
    let results = match (ImportApps { apps }).request() {
        Ok(results) => results,
        Err(e) => {
            eprintln!("Failed to import: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut by_id: std::collections::HashMap<u32, Result<ImportSummary, SamError>> =
        results.into_iter().collect();

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
                    error: Some(e.to_string()),
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
            return ExitCode::FAILURE;
        }
    }

    if any_failure {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_json<T: Serialize>(value: &T) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to serialize output: {}", e);
            ExitCode::FAILURE
        }
    }
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
