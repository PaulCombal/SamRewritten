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

//! Who is signed in. Fetched once at start-up and shared by the sidebar card
//! and the profile page.

use crate::backend::user_unlock_times::{AvatarImage, account_id};
use crate::gui_frontend::request::{GetCurrentUser, GetUserAvatar, GetUserPersonaName, Request};
use crate::utils::action_journal;
use gtk::gio::spawn_blocking;
use gtk::glib::MainContext;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct Identity {
    pub steam_id64: Cell<u64>,
    pub persona: RefCell<String>,
    pub avatar: RefCell<Option<AvatarImage>>,
}

pub(crate) type SharedIdentity = Rc<Identity>;

/// `on_step` runs as each field lands. Chained rather than fanned out: the
/// avatar fetch can poll for seconds inside the orchestrator, and that
/// connection is also serving the app list.
pub(crate) fn load_identity(identity: SharedIdentity, on_step: impl Fn(&Identity) + 'static) {
    MainContext::default().spawn_local(async move {
        let Ok(Ok(steam_id64)) = spawn_blocking(|| GetCurrentUser.request()).await else {
            return;
        };
        identity.steam_id64.set(steam_id64);
        // The journal only offers to undo what this account did.
        action_journal::set_account(account_id(steam_id64));
        on_step(&identity);

        if let Ok(Ok(Some(name))) =
            spawn_blocking(move || GetUserPersonaName { steam_id64 }.request()).await
        {
            *identity.persona.borrow_mut() = name;
            on_step(&identity);
        }

        if let Ok(Ok(Some(image))) =
            spawn_blocking(move || GetUserAvatar { steam_id64 }.request()).await
        {
            *identity.avatar.borrow_mut() = Some(image);
            on_step(&identity);
        }
    });
}
