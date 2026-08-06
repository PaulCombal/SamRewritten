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

use crate::backend::local_stats::read_schema_languages;
use crate::backend::orchestrator_client::{
    AppProgress, ExportApps, GetAchievementsAndStats, GetSubscribedAppList, ImportApps, LaunchApp,
    Request, ResetStats, SetAchievement, SetFloatStat, SetIntStat, StoreStatsAndAchievements,
    UnlockAllAchievements, set_orchestrator, shutdown_and_wait,
};
use crate::backend::stat_definitions::StatInfo;
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
        #[command(flatten)]
        language: Language,
    },
    /// List all stats defined for an app, with their current values, as JSON.
    ListStatistics {
        /// Steam AppID of the game to query.
        app_id: u32,
        #[command(flatten)]
        language: Language,
    },
    /// List the languages an app's schema offers for --language, as JSON.
    ListLanguages {
        /// Steam AppID of the game to query.
        app_id: u32,
    },
    /// List all apps owned by the logged-in Steam user as JSON.
    ListApps {
        /// Also include per-app achievement counts (total and unlocked).
        /// Slower: requires querying stats for every owned app.
        #[arg(long)]
        with_achievements: bool,
        /// Also include playtime and last-played time for every app.
        #[arg(long)]
        with_playtime: bool,
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
    /// Set the value of a stat for an app.
    SetStat {
        /// Steam AppID of the game.
        app_id: u32,
        /// Stat API name, as printed by `list-statistics`.
        stat_id: String,
        /// New value, an integer or a decimal depending on the stat's type.
        value: String,
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

#[derive(Args)]
struct Language {
    /// Steam schema language for achievement and stat names, e.g. 'french'.
    /// Defaults to the game's own language; see `list-languages`.
    #[arg(long)]
    language: Option<String>,
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
        Command::ListAchievements { app_id, language } => {
            match fetch_progress(app_id, language, "achievements") {
                Ok(progress) => print_json(&progress.achievements),
                Err(code) => code,
            }
        }

        Command::ListStatistics { app_id, language } => {
            match fetch_progress(app_id, language, "statistics") {
                Ok(progress) => print_json(&progress.stats),
                Err(code) => code,
            }
        }

        Command::ListLanguages { app_id } => print_json(&read_schema_languages(app_id)),

        Command::ListApps {
            with_achievements,
            with_playtime,
        } => {
            let apps = match (GetSubscribedAppList {
                include_playtime: with_playtime,
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
            Ok(true) => {
                println!("{}", json!({"success": true}));
                ExitCode::SUCCESS
            }
            Ok(false) => {
                eprintln!("Steam did not store the unlocked achievements");
                ExitCode::FAILURE
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
            Ok(true) => {
                println!("{}", json!({"success": true}));
                ExitCode::SUCCESS
            }
            Ok(false) => {
                eprintln!("Steam did not store the reset");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("Failed to reset all achievements: {e}");
                ExitCode::FAILURE
            }
        },

        Command::SetStat {
            app_id,
            stat_id,
            value,
        } => set_stat(app_id, stat_id, value),

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

/// An unreadable schema reads as empty and is left to the backend, which falls back
/// to the game's own language. That is also what a typo would silently produce, so
/// reject one here instead, while the list of real names is at hand.
fn resolve_language(app_id: u32, language: Option<String>) -> Result<String, String> {
    let Some(language) = language else {
        return Ok(String::new());
    };
    let offered = read_schema_languages(app_id);
    if offered.is_empty() || offered.iter().any(|l| l.eq_ignore_ascii_case(&language)) {
        Ok(language)
    } else {
        Err(format!(
            "App {app_id} has no '{language}' in its schema. Available: {}",
            offered.join(", ")
        ))
    }
}

fn fetch_progress(app_id: u32, language: Language, what: &str) -> Result<AppProgress, ExitCode> {
    let language = match resolve_language(app_id, language.language) {
        Ok(language) => language,
        Err(e) => {
            eprintln!("{e}");
            return Err(ExitCode::FAILURE);
        }
    };

    (GetAchievementsAndStats {
        app_id,
        launch: true,
        language,
    })
    .request()
    .map_err(|e| {
        eprintln!("Failed to get {what}: {e}");
        ExitCode::FAILURE
    })
}

/// The schema decides whether a stat is written as an integer or a float, so look the
/// stat up rather than making the caller declare it.
fn set_stat(app_id: u32, stat_id: String, value: String) -> ExitCode {
    let progress = match fetch_progress(app_id, Language { language: None }, "statistics") {
        Ok(progress) => progress,
        Err(code) => return code,
    };

    let Some(stat) = progress.stats.iter().find(|s| s.id() == stat_id) else {
        eprintln!("App {app_id} has no stat named {stat_id}");
        return ExitCode::FAILURE;
    };

    if (stat.permission() & 2) != 0 {
        eprintln!("Stat {stat_id} is protected by Steam and cannot be changed");
        return ExitCode::FAILURE;
    }

    let result = match stat {
        StatInfo::Integer(_) => match value.parse::<i32>() {
            Ok(value) => (SetIntStat {
                app_id,
                stat_id: stat_id.clone(),
                value,
            })
            .request(),
            Err(e) => {
                eprintln!("Stat {stat_id} takes an integer: {e}");
                return ExitCode::FAILURE;
            }
        },
        StatInfo::Float(_) => match value.parse::<f32>() {
            Ok(value) => (SetFloatStat {
                app_id,
                stat_id: stat_id.clone(),
                value,
            })
            .request(),
            Err(e) => {
                eprintln!("Stat {stat_id} takes a number: {e}");
                return ExitCode::FAILURE;
            }
        },
    };

    match result {
        Ok(true) => {
            println!("{}", json!({"id": stat_id, "success": true}));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("Failed to set stat {stat_id}: {other:?}");
            ExitCode::FAILURE
        }
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
        .unwrap_or(false);
        if !success {
            println!("Failed to {verb} achievement {id}");
        }
        results.push(AchievedResult { id, success });
    }

    // Every set above used `store: false`, so none of them exist outside
    // Steam's client until this returns true.
    match (StoreStatsAndAchievements { app_id }).request() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("Steam did not store the achievements");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("Failed to store stats and achievements: {e:?}");
            return ExitCode::FAILURE;
        }
    }

    print_json(&results)
}

fn export(app_ids: Vec<u32>) -> ExitCode {
    let results = match (ExportApps {
        app_ids: app_ids.clone(),
    })
    .request_with_progress(|done, total| eprintln!("Exported {done}/{total}"))
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
    let results = match (ImportApps { apps })
        .request_with_progress(|done, total| eprintln!("Imported {done}/{total}"))
    {
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
