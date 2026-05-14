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
use crate::utils::ipc_types::{
    AppAchievementExport, AppExport, AppStatExport, AppStatValue, ImportSummary, SamError,
};

/// Snapshot every achievement and stat for `app_id` into an `AppExport`.
/// `app_name` is left empty; callers that know the name fill it in.
pub fn collect_app_export(
    manager: &mut AppManager,
    app_id: u32,
) -> Result<AppExport, SamError> {
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
