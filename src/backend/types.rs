// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
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

use std::convert::TryFrom;

pub enum KeyValueEncoding {
    Utf8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatType {
    Invalid = 0,
    Integer = 1,
    Float = 2,
    AverageRate = 3,
    Achievements = 4,
    GroupAchievements = 5,
}

impl TryFrom<u8> for UserStatType {
    type Error = String;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UserStatType::Invalid),
            1 => Ok(UserStatType::Integer),
            2 => Ok(UserStatType::Float),
            3 => Ok(UserStatType::AverageRate),
            4 => Ok(UserStatType::Achievements),
            5 => Ok(UserStatType::GroupAchievements),
            _ => Err(format!("Invalid UserStatType value: {}", value)),
        }
    }
}

impl From<UserStatType> for u8 {
    fn from(stat_type: UserStatType) -> u8 {
        stat_type as u8
    }
}
