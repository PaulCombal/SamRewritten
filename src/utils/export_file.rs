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

use crate::utils::ipc_types::AppExport;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct ExportFile {
    pub format_version: u32,
    pub exported_at: String,
    pub apps: Vec<AppExport>,
}

/// Current UTC time as "YYYY-MM-DDTHH:MM:SSZ". Uses only std so the CLI build
/// (no glib) can produce the same format as the GUI.
pub fn iso8601_utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let total_days = secs.div_euclid(86400);
    let seconds_today = secs.rem_euclid(86400) as u32;
    let h = seconds_today / 3600;
    let m = (seconds_today % 3600) / 60;
    let s = seconds_today % 60;
    let (year, month, day) = days_since_epoch_to_ymd(total_days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

// Howard Hinnant's date algorithm.
fn days_since_epoch_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_epoch + 719468;
    let era = if days >= 0 { days } else { days - 146096 }.div_euclid(146097);
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970_01_01() {
        assert_eq!(days_since_epoch_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn known_dates() {
        // 2026-05-14 is 20587 days after 1970-01-01
        assert_eq!(days_since_epoch_to_ymd(20587), (2026, 5, 14));
        // 2000-02-29 (leap year)
        assert_eq!(days_since_epoch_to_ymd(11016), (2000, 2, 29));
    }
}
