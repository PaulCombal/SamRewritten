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

use crate::utils::bidir_child::BidirChild;
use crate::utils::ipc_types::{
    SamError, SteamResponse, read_frame_raw, read_message, write_message,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::process::ExitStatus;

/// Synchronous request/response client over a `BidirChild`. Owns the child
/// process and its two pipes, exposes a single round-trip API to callers, and
/// keeps all byte-level framing in one place.
pub struct IpcClient {
    pub child: BidirChild,
}

impl IpcClient {
    pub fn new(child: BidirChild) -> Self {
        Self { child }
    }

    pub fn send<T: Serialize + ?Sized>(&mut self, cmd: &T) -> Result<(), SamError> {
        write_message(&mut self.child.tx, cmd)
    }

    pub fn recv<R: DeserializeOwned>(&mut self) -> Result<R, SamError> {
        read_message(&mut self.child.rx)
    }
    
    pub fn recv_frame(&mut self) -> Result<Vec<u8>, SamError> {
        read_frame_raw(&mut self.child.rx)
    }

    pub fn request<R: DeserializeOwned, C: Serialize + ?Sized>(
        &mut self,
        cmd: &C,
    ) -> Result<R, SamError> {
        self.send(cmd)?;
        self.recv()
    }
    
    pub fn request_response<R: DeserializeOwned, C: Serialize + ?Sized>(
        &mut self,
        cmd: &C,
    ) -> Result<R, SamError> {
        let response: SteamResponse<R> = self.request(cmd)?;
        response.into()
    }

    pub fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.process.wait()
    }
}
