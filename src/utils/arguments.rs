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

use crate::dev_println;
use std::env;

#[derive(Debug)]
pub struct Arguments {
    pub is_orchestrator: bool,
    pub is_app: u32,
}
pub fn parse_arguments() -> Arguments {
    let mut args = Arguments {
        is_orchestrator: false,
        is_app: 0,
    };

    for (index, arg) in env::args().enumerate() {
        if index == 0 {
            // Self binary name
            continue;
        }

        match arg.as_str() {
            "--orchestrator" => {
                args.is_orchestrator = true;
                continue;
            }
            _ => {
                let split: Vec<&str> = arg.split("=").collect();
                if split.len() != 2 {
                    continue;
                }

                let key = split[0];
                let value = split[1];

                if value.len() == 0 {
                    continue;
                }

                if key == "--app" {
                    args.is_app = value.parse::<u32>().unwrap();
                }
            }
        }
    }

    dev_println!("New process launched with arguments: {:?}", args);

    args
}
