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

/// Wall-clock time of day as `HH:MM:SS` in UTC.
///
/// UTC keeps this dependency-free: `std` exposes no local timezone offset. The
/// timestamps exist to gauge elapsed time between log lines, not the time of day,
/// so the constant offset from local time is irrelevant.
pub fn log_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!(
        "{:02}:{:02}:{:02}",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}

/// Debug-only logging. `dev_println!("SCOPE", "msg {}", x)` prints
/// `[SCOPE HH:MM:SS] msg ...`; a single-argument call omits the scope.
#[macro_export]
macro_rules! dev_println {
    ($scope:literal, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            println!("[{}\t{}] {}", $scope, $crate::utils::dev_println::log_timestamp(), format_args!($($arg)+));
        }
    };
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!("[{}] {}", $crate::utils::dev_println::log_timestamp(), format_args!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! dev_print {
    ($scope:literal, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            print!("[{}\t{}] {}", $scope, $crate::utils::dev_println::log_timestamp(), format_args!($($arg)+));
        }
    };
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            print!("[{}] {}", $crate::utils::dev_println::log_timestamp(), format_args!($($arg)*));
        }
    };
}
