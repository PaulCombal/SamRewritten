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

use crate::utils::ipc_types::SamError;
#[cfg(unix)]
use interprocess::unnamed_pipe::pipe;
use interprocess::unnamed_pipe::{Recver, Sender};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::process::{Child, Command};

#[derive(Debug)]
pub struct BidirChild {
    pub process: Child,
    pub tx: Sender,
    pub rx: Recver,
}

impl BidirChild {
    #[cfg(unix)]
    pub fn new(command: &mut Command) -> Result<Self, SamError> {
        let make_pipe = || {
            pipe().map_err(|e| {
                eprintln!("Unable to create a pipe: {e}");
                SamError::SocketCommunicationFailed
            })
        };
        let (parent_to_child_tx, parent_to_child_rx) = make_pipe()?;
        let (child_to_parent_tx, child_to_parent_rx) = make_pipe()?;
        
        let process = match command
            .arg(format!("--tx={}", child_to_parent_tx.as_raw_fd()))
            .arg(format!("--rx={}", parent_to_child_rx.as_raw_fd()))
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Unable to spawn a child process: {e}");
                return Err(SamError::UnknownError);
            }
        };

        Ok(Self {
            process,
            tx: parent_to_child_tx,
            rx: child_to_parent_rx,
        })
    }

    #[cfg(windows)]
    pub fn new(command: &mut Command) -> Result<Self, SamError> {
        let make_pipe = || {
            interprocess::os::windows::unnamed_pipe::CreationOptions::default()
                .inheritable(true)
                .build()
                .map_err(|e| {
                    eprintln!("Unable to create a pipe: {e}");
                    SamError::SocketCommunicationFailed
                })
        };
        let (parent_to_child_tx, parent_to_child_rx) = make_pipe()?;
        let (child_to_parent_tx, child_to_parent_rx) = make_pipe()?;

        let child_to_parent_tx_handle: OwnedHandle = child_to_parent_tx.into();
        let parent_to_child_rx_handle: OwnedHandle = parent_to_child_rx.into();

        let process = match {
            command
                .arg(format!(
                    "--tx={}",
                    child_to_parent_tx_handle.as_raw_handle() as usize
                ))
                .arg(format!(
                    "--rx={}",
                    parent_to_child_rx_handle.as_raw_handle() as usize
                ))
                .spawn()
        } {
            Ok(child) => {
                drop(parent_to_child_rx_handle);
                drop(child_to_parent_tx_handle);

                child
            }
            Err(e) => {
                eprintln!("Unable to spawn a child process: {e}");
                return Err(SamError::UnknownError);
            }
        };

        Ok(Self {
            process,
            tx: parent_to_child_tx,
            rx: child_to_parent_rx,
        })
    }
}
