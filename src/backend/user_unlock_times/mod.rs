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

//! A Steam account's per-achievement unlock times and the social/friends queries
//! that back copy-timing mode.
//!
//! - [`unlock_times`]: a single bulk parse of the on-disk
//!   `UserGameStats_<account_id>_<app_id>.bin` cache joined against the shared
//!   `UserGameStatsSchema_<app_id>.bin` — the same join `local_stats.rs` does for
//!   counts, extended to pull each group's `AchievementTimes`. Plus the cache path
//!   helpers used to locate those files.
//! - [`friends`]: native friends-interface queries — the live friends list,
//!   per-friend persona name, and avatar (RGBA).

mod friends;
mod unlock_times;

pub use friends::*;
pub use unlock_times::*;
