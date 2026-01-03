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

#[inline]
#[allow(dead_code)]
pub fn format_seconds_to_mm_ss(total_seconds: usize) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}", minutes, seconds)
}

#[inline]
pub fn format_seconds_to_hh_mm_ss(total_seconds: usize) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_seconds_to_mm_ss() {
        assert_eq!(format_seconds_to_mm_ss(0), "00:00");
        assert_eq!(format_seconds_to_mm_ss(59), "00:59");
        assert_eq!(format_seconds_to_mm_ss(60), "01:00");
        assert_eq!(format_seconds_to_mm_ss(3599), "59:59");
        assert_eq!(format_seconds_to_mm_ss(3600), "60:00");
    }

    #[test]
    fn test_format_seconds_to_hh_mm_ss() {
        assert_eq!(format_seconds_to_hh_mm_ss(0), "00:00");
        assert_eq!(format_seconds_to_hh_mm_ss(59), "00:59");
        assert_eq!(format_seconds_to_hh_mm_ss(60), "01:00");
        assert_eq!(format_seconds_to_hh_mm_ss(3599), "59:59");
        assert_eq!(format_seconds_to_hh_mm_ss(3600), "01:00:00");
        assert_eq!(format_seconds_to_hh_mm_ss(3661), "01:01:01");
        assert_eq!(format_seconds_to_hh_mm_ss(86399), "23:59:59");
        assert_eq!(format_seconds_to_hh_mm_ss(86400), "24:00:00");
    }
}
