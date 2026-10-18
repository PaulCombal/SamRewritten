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

// Every IClient* slot is an IPC stub that loads its own method name with a
// RIP-relative `lea` before dispatching, so a slot can be found by name when a
// Steam update inserts methods ahead of it. Bytes are scanned unaligned, so a
// match may be a false decode pointing anywhere: all reads go through the
// kernel, which fails cleanly on unmapped memory instead of faulting.

use std::os::raw::c_void;

const STUB_SCAN_LEN: usize = 0x400;

/// Slot within `radius` of `expected` whose stub names `method`, nearest first.
pub unsafe fn find_slot_by_name(
    vtable: *const *const c_void,
    expected: usize,
    radius: usize,
    method: &str,
) -> Option<usize> {
    let mut needle = method.as_bytes().to_vec();
    needle.push(0);

    let mut candidates = std::iter::once(expected).chain((1..=radius).flat_map(|d| {
        [expected.checked_add(d), expected.checked_sub(d)]
            .into_iter()
            .flatten()
    }));

    candidates.find(|&slot| {
        let mut entry = [0u8; size_of::<usize>()];
        if !read_own_memory(vtable as usize + slot * size_of::<usize>(), &mut entry) {
            return false;
        }
        let stub = usize::from_ne_bytes(entry);
        stub != 0 && stub_names(stub, &needle)
    })
}

fn stub_names(stub: usize, needle: &[u8]) -> bool {
    let mut code = vec![0u8; STUB_SCAN_LEN];
    if !read_own_memory(stub, &mut code) {
        return false;
    }

    // A stub opens with the interface name then its own method name; looking
    // further would run into whatever function follows it in memory.
    let mut found = vec![0u8; needle.len()];
    code.windows(7)
        .enumerate()
        .filter(|(_, insn)| {
            // lea r64, [rip + disp32]: REX.W (optionally REX.R), 8D, ModRM mod=00 rm=101
            matches!(insn[0], 0x48 | 0x4c) && insn[1] == 0x8d && insn[2] & 0xc7 == 0x05
        })
        .take(2)
        .any(|(offset, insn)| {
            let disp = i32::from_le_bytes([insn[3], insn[4], insn[5], insn[6]]) as isize;
            let target = (stub + offset + 7).wrapping_add_signed(disp);
            read_own_memory(target, &mut found) && found == needle
        })
}

// Not /proc/self/mem or process_vm_readv: the Snap's AppArmor and seccomp
// profiles deny both. A pipe write from a bad address just returns EFAULT.
#[cfg(unix)]
fn read_own_memory(addr: usize, buf: &mut [u8]) -> bool {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    }
    let Ok((mut reader, writer)) = std::io::pipe() else {
        return false;
    };
    let written = unsafe { write(writer.as_raw_fd(), addr as *const c_void, buf.len()) };
    drop(writer);
    written == buf.len() as isize && reader.read_exact(buf).is_ok()
}

#[cfg(windows)]
fn read_own_memory(addr: usize, buf: &mut [u8]) -> bool {
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn ReadProcessMemory(
            process: *mut c_void,
            base: *const c_void,
            buf: *mut c_void,
            size: usize,
            read: *mut usize,
        ) -> i32;
    }
    let mut read = 0usize;
    unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const c_void,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut read,
        ) != 0
            && read == buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STUB_LEN: usize = 64;

    /// Strings then back-to-back stubs in one buffer, so every `lea` reaches
    /// its target and a scan that overran its stub would see the next one.
    fn image(methods: &[&str]) -> (Vec<u8>, Vec<usize>) {
        let mut buf = b"IClientUser\0".to_vec();
        let method_at: Vec<usize> = methods
            .iter()
            .map(|m| {
                let at = buf.len();
                buf.extend_from_slice(m.as_bytes());
                buf.push(0);
                at
            })
            .collect();
        let mut stubs = Vec::new();
        for &target in &method_at {
            let stub = buf.len();
            buf.resize(stub + STUB_LEN, 0x90);
            for (lea_at, target) in [(stub + 20, 0), (stub + 34, target)] {
                let disp = target as i32 - (lea_at as i32 + 7);
                buf[lea_at..lea_at + 3].copy_from_slice(&[0x4c, 0x8d, 0x05]);
                buf[lea_at + 3..lea_at + 7].copy_from_slice(&disp.to_le_bytes());
            }
            stubs.push(stub);
        }
        buf.resize(buf.len() + STUB_SCAN_LEN, 0);
        (buf, stubs)
    }

    #[test]
    fn reads_mapped_memory_and_rejects_unmapped() {
        let data = *b"GetS";
        let mut buf = [0u8; 4];
        assert!(read_own_memory(data.as_ptr() as usize, &mut buf));
        assert_eq!(buf, data);
        assert!(!read_own_memory(0, &mut buf));
        assert!(!read_own_memory(usize::MAX - 2, &mut buf));
    }

    #[test]
    fn finds_slot_nearest_expected_without_overrunning_stubs() {
        let (buf, stubs) = image(&[
            "BIsSubscribedApp",
            "BIsSubscribedApp",
            "GetSubscribedApps",
            "AckSystemIM",
        ]);
        let vtable: Vec<*const c_void> = stubs.iter().map(|&s| buf[s..].as_ptr().cast()).collect();

        let found = unsafe { find_slot_by_name(vtable.as_ptr(), 1, 2, "GetSubscribedApps") };
        assert_eq!(found, Some(2));
        let out_of_reach = unsafe { find_slot_by_name(vtable.as_ptr(), 0, 1, "GetSubscribedApps") };
        assert_eq!(out_of_reach, None);
    }
}
