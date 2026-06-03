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

//! gettext localisation over GLib's `dgettext` (no extra crate). The English
//! source string is the catalogue key. `build.rs` compiles each `po/<lang>.po`
//! to `<locale_dir>/<lang>/LC_MESSAGES/samrewritten.mo`; see `po/README.md`.

use gtk::glib::{self, GString};
use std::env;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Once;

pub const TEXT_DOMAIN: &str = "samrewritten";

/// `(locale code, native name)`; add a row per new `po/<code>.po`. "en" has no
/// catalogue — it is the source language, so gettext falls back to the msgids.
pub const LANGUAGES: &[(&str, &str)] = &[("en", "English"), ("fr", "Français")];

// glib-rs wraps dgettext/dngettext but not bindtextdomain; the symbol lives in
// libintl, which GTK already links.
unsafe extern "C" {
    fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domainname: *const c_char, codeset: *const c_char) -> *mut c_char;
}

/// Bind the gettext domain to our bundled catalogues. Must run after GTK's
/// `setlocale(LC_ALL, "")` (i.e. from the `command_line`/`activate` handler). A
/// system install has no bundled dir and falls through to gettext's default path.
pub fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let Some(dir) = locale_dir() else {
            return;
        };
        let (Ok(domain), Ok(dir), Ok(codeset)) = (
            CString::new(TEXT_DOMAIN),
            CString::new(dir),
            CString::new("UTF-8"),
        ) else {
            return;
        };
        unsafe {
            bindtextdomain(domain.as_ptr(), dir.as_ptr());
            bind_textdomain_codeset(domain.as_ptr(), codeset.as_ptr());
        }
    });
}

/// Select the UI language via `LANGUAGE` (empty = follow system locale). gettext
/// reads it lazily on first lookup, so call once at start-up; a change applies
/// only on the next launch.
pub fn set_language(code: &str) {
    // g_setenv also updates the C runtime environment, so GTK's bundled gettext
    // sees the change on Windows; std::env::set_var (SetEnvironmentVariableW)
    // would not reach the CRT getenv() that libintl reads.
    // SAFETY: runs on the main thread at start-up, before workers spawn.
    unsafe {
        if code.is_empty() {
            glib::unsetenv("LANGUAGE");
        } else {
            let _ = glib::setenv("LANGUAGE", code, true);
        }
    }
}

#[inline]
pub fn tr(msgid: &str) -> GString {
    glib::dgettext(Some(TEXT_DOMAIN), msgid)
}

#[inline]
#[allow(dead_code)]
pub fn trn(singular: &str, plural: &str, n: u64) -> GString {
    glib::dngettext(Some(TEXT_DOMAIN), singular, plural, n as _)
}

/// Mark a literal for extraction without translating now (gettext's `N_()`),
/// translated later via [`tr`]. Needs `--keyword=tr_noop` in xgettext.
#[inline]
pub const fn tr_noop(msgid: &str) -> &str {
    msgid
}

/// Resolve the directory holding `<lang>/LC_MESSAGES/samrewritten.mo`, mirroring
/// the packaging branches in [`crate::gui_frontend::gsettings::get_settings`].
fn locale_dir() -> Option<String> {
    // AppImage: cargo-appimage flattens assets to the AppDir root.
    if let Ok(appdir) = env::var("APPDIR") {
        return Some(format!("{appdir}/locale"));
    }
    // Snap real install (dev builds use a different SNAP_NAME and fall through).
    if let Ok(snap) = env::var("SNAP")
        && env::var("SNAP_NAME").as_deref() == Ok("samrewritten")
    {
        return Some(format!("{snap}/usr/share/locale"));
    }
    if let Ok(dir) = env::var("SAM_LOCALE_DIR_FALLBACK") {
        return Some(dir);
    }
    // Dev: ./locale produced by build.rs when running from the tree.
    if Path::new("./locale").is_dir() {
        return Some("./locale".to_owned());
    }
    // Windows: catalogues sit in share/locale next to the exe (no default path).
    #[cfg(windows)]
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let locale = dir.join("share").join("locale");
        if locale.is_dir() {
            return locale.to_str().map(str::to_owned);
        }
    }
    // System install: gettext's default search path.
    None
}
