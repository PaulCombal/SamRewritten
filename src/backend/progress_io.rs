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

use crate::backend::app_manager::AppManager;
use crate::backend::stat_definitions::StatInfo;
use crate::utils::app_paths::get_executable_path;
use crate::utils::bidir_child::BidirChild;
use crate::utils::ipc_client::IpcClient;
pub use crate::utils::ipc_types::parse_response_bytes;
use crate::utils::ipc_types::{
    AppAchievementExport, AppExport, AppStatExport, AppStatValue, ImportSummary, SamError,
    SteamCommand,
};
use std::process::Command;
use std::sync::{Arc, Mutex};

pub const MAX_CONCURRENT_APPS: usize = 30;

/// Spawn one short-lived `samrewritten --app=X` child per `(app_id, command)`
/// item, run them in parallel with up to `max_concurrent` workers continuously
/// pulling from a shared queue, and return the per-app raw response bytes (or
/// a `SamError` if the worker failed).
///
/// Callers deserialize the response bytes themselves via `SteamResponse::<T>`.
/// The helper is intentionally generic — the same machinery serves
/// export/import today and will serve bulk unlock/lock next.
pub fn run_command_on_apps_concurrent(
    items: Vec<(u32, SteamCommand)>,
    max_concurrent: usize,
    progress: Option<Arc<dyn Fn(usize, usize, u32) + Send + Sync>>,
) -> Vec<(u32, Result<Vec<u8>, SamError>)> {
    let total = items.len();
    if total == 0 {
        return Vec::new();
    }
    let cap = max_concurrent.max(1).min(MAX_CONCURRENT_APPS).min(total);

    let queue = Arc::new(Mutex::new(items.into_iter()));
    let results = Arc::new(Mutex::new(
        Vec::<(u32, Result<Vec<u8>, SamError>)>::with_capacity(total),
    ));
    let done = Arc::new(Mutex::new(0usize));

    std::thread::scope(|s| {
        for _ in 0..cap {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let done = Arc::clone(&done);
            let progress = progress.clone();
            s.spawn(move || {
                loop {
                    let next = queue.lock().unwrap().next();
                    let Some((app_id, command)) = next else { break };
                    let outcome = run_one(app_id, command);
                    let step = {
                        let mut d = done.lock().unwrap();
                        *d += 1;
                        *d
                    };
                    if let Some(cb) = &progress {
                        cb(step, total, app_id);
                    }
                    results.lock().unwrap().push((app_id, outcome));
                }
            });
        }
    });

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}

fn run_one(app_id: u32, command: SteamCommand) -> Result<Vec<u8>, SamError> {
    let current_exe = get_executable_path();
    let child = BidirChild::new(Command::new(current_exe).arg(format!("--app={app_id}")))?;
    let mut ipc = IpcClient::new(child);

    std::thread::sleep(std::time::Duration::from_millis(10));

    let response = ipc.send(&command).and_then(|_| ipc.recv_frame());

    std::thread::sleep(std::time::Duration::from_millis(10));

    let _ = ipc.send(&SteamCommand::Shutdown);
    let _ = ipc.recv_frame();
    let _ = ipc.wait();

    response
}

/// Snapshot every achievement and stat for `app_id` into an `AppExport`.
/// `app_name` is left empty; callers that know the name fill it in.
pub fn collect_app_export(manager: &mut AppManager, app_id: u32) -> Result<AppExport, SamError> {
    let achievements = manager.get_achievements(false)?;
    let stats = manager.get_statistics()?;

    Ok(AppExport {
        app_id,
        app_name: String::new(),
        achievements: achievements
            .into_iter()
            .map(|a| AppAchievementExport {
                id: a.id,
                is_achieved: a.is_achieved,
                permission: a.permission,
            })
            .collect(),
        stats: stats
            .into_iter()
            .map(|s| match s {
                StatInfo::Integer(i) => AppStatExport {
                    id: i.id,
                    value: AppStatValue::Int(i.int_value),
                    permission: i.permission,
                },
                StatInfo::Float(f) => AppStatExport {
                    id: f.id,
                    value: AppStatValue::Float(f.float_value),
                    permission: f.permission,
                },
            })
            .collect(),
    })
}

/// Apply an `AppExport` payload through `manager`. Protected fields
/// (stat permission & 2, achievement permission != 0) are skipped
/// client-side. Stats are written before achievements; a single
/// `store_stats_and_achievements()` is issued at the end.
pub fn apply_app_export(manager: &AppManager, payload: AppExport) -> ImportSummary {
    let mut summary = ImportSummary::default();

    for stat in payload.stats {
        if (stat.permission & 2) != 0 {
            summary.skipped_protected.push(format!("stat:{}", stat.id));
            continue;
        }
        let res = match stat.value {
            AppStatValue::Int(v) => manager.set_stat_i32(&stat.id, v),
            AppStatValue::Float(v) => manager.set_stat_f32(&stat.id, v),
        };
        match res {
            Ok(_) => summary.stats_applied += 1,
            Err(e) => summary
                .errors
                .push(format!("stat:{} failed: {}", stat.id, e)),
        }
    }

    for ach in payload.achievements {
        if ach.permission != 0 {
            summary.skipped_protected.push(format!("ach:{}", ach.id));
            continue;
        }
        match manager.set_achievement(&ach.id, ach.is_achieved, false) {
            Ok(_) => summary.achievements_applied += 1,
            Err(e) => summary.errors.push(format!("ach:{} failed: {}", ach.id, e)),
        }
    }

    if let Err(e) = manager.store_stats_and_achievements() {
        summary.errors.push(format!("store failed: {}", e));
    }

    summary
}
