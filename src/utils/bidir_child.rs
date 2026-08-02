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

use crate::utils::inherit::keep_private;
#[cfg(unix)]
use crate::utils::inherit::set_inheritable;
use crate::utils::ipc_types::SamError;
#[cfg(unix)]
use interprocess::unnamed_pipe::pipe;
use interprocess::unnamed_pipe::{Recver, Sender};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::process::{Child, Command};
use std::sync::Mutex;

#[derive(Debug)]
pub struct BidirChild {
    pub process: Child,
    pub tx: Sender,
    pub rx: Recver,
}

/// Ends are born inheritable — on unix until `keep_private` runs, on Windows the
/// child's two for all of `spawn`. Both platforms need this: without it a
/// concurrent spawn captures ends that aren't its own, and their owner's reader
/// then never sees EOF.
static SPAWN_LOCK: Mutex<()> = Mutex::new(());

impl BidirChild {
    #[cfg(unix)]
    pub fn new(command: &mut Command) -> Result<Self, SamError> {
        let _spawn_guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let make_pipe = || {
            pipe().map_err(|e| {
                eprintln!("Unable to create a pipe: {e}");
                SamError::SocketCommunicationFailed
            })
        };
        let (parent_to_child_tx, parent_to_child_rx) = make_pipe()?;
        let (child_to_parent_tx, child_to_parent_rx) = make_pipe()?;

        let child_tx = child_to_parent_tx.as_raw_fd();
        let child_rx = parent_to_child_rx.as_raw_fd();
        keep_private(&parent_to_child_tx);
        keep_private(&parent_to_child_rx);
        keep_private(&child_to_parent_tx);
        keep_private(&child_to_parent_rx);

        // SAFETY: runs in the forked child before exec, and `set_inheritable` is
        // one `fcntl` call, so it is async-signal-safe. Failing aborts the spawn:
        // the child would otherwise reconstruct whatever else lands on those fds.
        unsafe {
            command.pre_exec(move || {
                set_inheritable(child_tx, true)?;
                set_inheritable(child_rx, true)
            });
        }

        let process = match command
            .arg(format!("--tx={child_tx}"))
            .arg(format!("--rx={child_rx}"))
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
        let _spawn_guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());

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

        keep_private(&parent_to_child_tx);
        keep_private(&child_to_parent_rx);

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
