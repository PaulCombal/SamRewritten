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
use interprocess::unnamed_pipe::{Recver, Sender};
use std::cell::Cell;
use std::env;
#[cfg(unix)]
use std::os::fd::FromRawFd;
#[cfg(windows)]
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::process::exit;
use std::rc::Rc;

#[derive(Debug)]
pub struct CliArguments {
    pub is_orchestrator: bool,
    pub is_app: u32,
    pub rx: Option<Recver>,
    pub tx: Option<Sender>,
}

#[derive(Debug)]
pub struct GuiArguments {
    pub auto_open: Rc<Cell<u32>>,
}

pub fn parse_cli_arguments() -> CliArguments {
    let mut args = CliArguments {
        is_orchestrator: false,
        is_app: 0,
        rx: None,
        tx: None,
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
            _ => unsafe {
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
                    continue;
                }

                #[cfg(target_os = "linux")]
                if key == "--tx" {
                    let raw_handle = value.parse::<i32>().expect("Invalid value for --tx");
                    args.tx = Some(Sender::from_raw_fd(raw_handle));
                    continue;
                }

                #[cfg(target_os = "windows")]
                if key == "--tx" {
                    let raw_handle =
                        value.parse::<usize>().expect("Invalid value for --tx") as RawHandle;
                    args.tx = Some(Sender::from_raw_handle(raw_handle));
                    continue;
                }

                #[cfg(target_os = "linux")]
                if key == "--rx" {
                    let raw_handle = value.parse::<i32>().expect("Invalid value for --rx");
                    args.rx = Some(Recver::from_raw_fd(raw_handle));
                    continue;
                }

                #[cfg(target_os = "windows")]
                if key == "--rx" {
                    let raw_handle =
                        value.parse::<usize>().expect("Invalid value for --rx") as RawHandle;
                    args.rx = Some(Recver::from_raw_handle(raw_handle));
                    continue;
                }
            },
        }
    }

    if args.tx.is_some() != args.rx.is_some() {
        eprintln!("Invalid arguments, tx and rx must be provided.");
        exit(1);
    }

    dev_println!("[PID {}] New process launched with arguments: {:?}", std::process::id(), args);

    args
}

#[cfg(not(feature = "cli"))]
pub fn parse_gui_arguments(cmd_line: &gtk::gio::ApplicationCommandLine) -> GuiArguments {
    use gtk::prelude::ApplicationCommandLineExt;

    let arguments = cmd_line.arguments();
    let args = GuiArguments {
        auto_open: Rc::new(Cell::new(0)),
    };

    for arg in arguments.iter().skip(1) {
        // Skip the first argument (program name)
        if let Some(arg_str) = arg.to_str() {
            if arg_str.starts_with("--auto-open=") {
                if let Some(value_str) = arg_str.strip_prefix("--auto-open=") {
                    if let Ok(value) = value_str.parse::<u32>() {
                        args.auto_open.set(value);
                        println!("Parsed --auto-open value: {}", value);
                    } else {
                        eprintln!("Error: Invalid value for --auto-open: {}", value_str);
                    }
                }
            }
        }
    }

    args
}
