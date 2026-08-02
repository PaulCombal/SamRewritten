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

//! Control over which pipe ends a spawned process inherits.
//!
//! `interprocess` hands back inheritable pipe ends on both platforms, so a child
//! otherwise receives a copy of *our* ends too. The damaging one is the write end
//! of the pipe we read from: while any copy of it stays open, our read can never
//! report EOF, so a parent that dies without sending `Shutdown` leaves us blocked
//! in `read` forever instead of taking the parent-pipe-error path.

use std::io;

#[cfg(unix)]
mod imp {
    use std::io;
    use std::os::fd::RawFd;
    use std::os::raw::c_int;

    const F_SETFD: c_int = 2;
    const FD_CLOEXEC: c_int = 1;

    unsafe extern "C" {
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    }

    pub fn set_inheritable(fd: RawFd, inheritable: bool) -> io::Result<()> {
        let flag = if inheritable { 0 } else { FD_CLOEXEC };
        if unsafe { fcntl(fd, F_SETFD, flag) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::io::RawHandle;

    const HANDLE_FLAG_INHERIT: u32 = 1;

    unsafe extern "system" {
        fn SetHandleInformation(handle: RawHandle, mask: u32, flags: u32) -> i32;
    }

    pub fn set_inheritable(handle: RawHandle, inheritable: bool) -> io::Result<()> {
        let flags = if inheritable { HANDLE_FLAG_INHERIT } else { 0 };
        if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, flags) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

pub use imp::set_inheritable;

/// Best effort — this process keeps working without the flag, but whoever reads
/// the other end of that pipe loses its shutdown-on-parent-death signal.
#[cfg(unix)]
pub fn keep_private(io: &impl std::os::fd::AsRawFd) {
    warn_on_err(set_inheritable(io.as_raw_fd(), false));
}

#[cfg(windows)]
pub fn keep_private(io: &impl std::os::windows::io::AsRawHandle) {
    warn_on_err(set_inheritable(io.as_raw_handle(), false));
}

fn warn_on_err(result: io::Result<()>) {
    if let Err(e) = result {
        eprintln!("Unable to withhold a pipe end from child processes: {e}");
    }
}
