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

//! Modal friend picker for copy-timing mode: an omnibar over a *virtualized* list
//! of friends (avatar + name). Only visible rows build widgets / load avatars, so
//! a large friends list stays cheap. Selecting a user is only possible by clicking
//! a result; a pasted SteamID64 surfaces as its own clickable result.

use crate::backend::user_unlock_times::{Friend, STEAMID64_BASE};
use crate::gui_frontend::gobjects::friend::GFriendObject;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{GetUserAvatar, GetUserPersonaName, Request};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use crate::utils::ipc_types::SamError;
use gtk::gdk::{MemoryFormat, MemoryTexture};
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib;
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, ClosureExpression, CustomFilter, Entry, FilterChange, FilterListModel,
    Image, Label, ListItem, ListView, NoSelection, Orientation, Revealer, ScrolledWindow,
    SignalListItemFactory, Spinner, Widget, Window,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

/// Open the modal and run `on_select` for the clicked friend. `on_select` loads
/// the friend's data (it returns a future) while the picker stays open with a
/// loading state; on success the window closes, on error a banner is shown so the
/// user can pick someone else. `on_clear` drops the current selection (its button
/// is enabled only when `has_selection`).
pub fn open_friend_picker<Fut>(
    parent: Option<&Window>,
    friends: Vec<Friend>,
    has_selection: bool,
    on_clear: impl Fn() + 'static,
    on_select: impl Fn(Friend) -> Fut + 'static,
) where
    Fut: Future<Output = Result<(), SamError>> + 'static,
{
    let window = Window::builder()
        .modal(true)
        .title(tr("Copy timing from a friend").as_str())
        .destroy_with_parent(true)
        .default_width(420)
        .default_height(520)
        .build();
    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    // Index 0 is the "use the pasted SteamID64" row; its steam-id tracks the query.
    let store = ListStore::new::<GFriendObject>();
    let custom = GFriendObject::custom();
    store.append(&custom);
    for f in &friends {
        store.append(&GFriendObject::new(f));
    }

    let query: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let filter = CustomFilter::new(clone!(
        #[strong]
        query,
        move |obj| {
            let Some(item) = obj.downcast_ref::<GFriendObject>() else {
                return false;
            };
            let q = query.borrow();
            let q = q.trim();
            if item.is_custom() {
                return q
                    .parse::<u64>()
                    .map(|v| v >= STEAMID64_BASE)
                    .unwrap_or(false);
            }
            q.is_empty() || item.search_text().contains(&q.to_lowercase())
        }
    ));
    let filter_model = FilterListModel::new(Some(store), Some(filter.clone()));
    let selection = NoSelection::new(Some(filter_model));

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, list_item| {
        let li = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");

        let avatar = ShimmerImage::new();
        avatar.set_size_request(36, 36);
        let name = Label::builder().halign(Align::Start).build();
        let id = Label::builder()
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .build();
        let text = Box::builder().orientation(Orientation::Vertical).build();
        text.append(&name);
        text.append(&id);
        let hbox = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(6)
            .margin_end(6)
            .build();
        hbox.append(&avatar);
        hbox.append(&text);
        li.set_child(Some(&hbox));

        li.property_expression("item")
            .chain_property::<GFriendObject>("avatar-url")
            .bind(&avatar, "url", Widget::NONE);

        // SteamID label (blank for an empty custom row).
        let id_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let v = values.get(1).and_then(|v| v.get::<u64>().ok()).unwrap_or(0);
            let s = if v == 0 { String::new() } else { v.to_string() };
            Some(s.to_value())
        });
        ClosureExpression::new::<String>(
            &[li.property_expression("item")
                .chain_property::<GFriendObject>("steam-id")],
            id_closure,
        )
        .bind(&id, "label", Widget::NONE);

        // Name (the custom row shows a fixed prompt instead).
        let name_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let custom = values
                .get(1)
                .and_then(|v| v.get::<bool>().ok())
                .unwrap_or(false);
            let n = values
                .get(2)
                .and_then(|v| v.get::<String>().ok())
                .unwrap_or_default();
            let label = if custom {
                if n.is_empty() {
                    tr("Use the pasted SteamID64").to_string()
                } else {
                    n
                }
            } else {
                n
            };
            Some(label.to_value())
        });
        ClosureExpression::new::<String>(
            &[
                li.property_expression("item")
                    .chain_property::<GFriendObject>("is-custom"),
                li.property_expression("item")
                    .chain_property::<GFriendObject>("name"),
            ],
            name_closure,
        )
        .bind(&name, "label", Widget::NONE);
    });

    let list_view = ListView::builder()
        .model(&selection)
        .factory(&factory)
        .single_click_activate(true)
        .build();

    // Error banner (a Revealer-wrapped row), shown when a selection fails to load.
    // Avoids the deprecated gtk::InfoBar; styling comes from stock classes.
    let banner_label = Label::builder()
        .halign(Align::Start)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    let banner_close = Button::builder()
        .icon_name("window-close-symbolic")
        .has_frame(false)
        .valign(Align::Center)
        .build();
    let banner_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .css_classes(["error"])
        .margin_top(6)
        .margin_bottom(6)
        .build();
    banner_box.append(
        &Image::builder()
            .icon_name("dialog-error-symbolic")
            .valign(Align::Center)
            .margin_start(8)
            .build(),
    );
    banner_box.append(&banner_label);
    banner_box.append(&banner_close);
    let banner = Revealer::builder()
        .child(&banner_box)
        .reveal_child(false)
        .build();
    banner_close.connect_clicked(clone!(
        #[weak]
        banner,
        move |_| banner.set_reveal_child(false)
    ));

    // Spinner shown next to the search box while a selection is loading.
    let spinner = Spinner::builder().valign(Align::Center).build();
    spinner.set_visible(false);

    let on_select = Rc::new(on_select);
    list_view.connect_activate(clone!(
        #[weak]
        window,
        #[weak]
        banner,
        #[weak]
        banner_label,
        #[weak]
        spinner,
        #[strong]
        on_select,
        move |lv, pos| {
            let Some(item) = lv.model().and_then(|m| m.item(pos)) else {
                return;
            };
            let Some(f) = item.downcast_ref::<GFriendObject>() else {
                return;
            };
            if f.is_custom() && f.steam_id() == 0 {
                return;
            }

            // Load while the picker stays open: disable input, show the spinner,
            // and either close on success or surface the error in the banner.
            banner.set_reveal_child(false);
            spinner.set_visible(true);
            spinner.start();
            lv.set_sensitive(false);
            let fut = on_select(f.to_friend());
            MainContext::default().spawn_local(clone!(
                #[weak]
                window,
                #[weak]
                banner,
                #[weak]
                banner_label,
                #[weak]
                spinner,
                #[weak]
                lv,
                async move {
                    let result = fut.await;
                    spinner.stop();
                    spinner.set_visible(false);
                    lv.set_sensitive(true);
                    match result {
                        Ok(()) => window.close(),
                        Err(e) => {
                            let msg = if e == SamError::ProfilePrivate {
                                tr("This profile is private")
                            } else {
                                tr("Couldn't load this user")
                            };
                            banner_label.set_label(&msg);
                            banner.set_reveal_child(true);
                        }
                    }
                }
            ));
        }
    ));

    let search = Entry::builder()
        .placeholder_text(tr("Search friends or paste a SteamID64").as_str())
        .primary_icon_name("system-search-symbolic")
        .hexpand(true)
        .build();
    // Resolve a pasted SteamID64's persona name so the custom row shows who it
    // is; `last_resolved` guards against re-fetching the same id on every
    // keystroke, and resets when the id becomes incomplete so re-entering it
    // fetches again.
    let last_resolved: Rc<Cell<u64>> = Rc::new(Cell::new(0));
    search.connect_changed(clone!(
        #[strong]
        query,
        #[weak]
        filter,
        #[strong]
        custom,
        #[strong]
        last_resolved,
        move |e| {
            let text = e.text().to_string();
            let id = text.trim().parse::<u64>().unwrap_or(0);
            custom.set_steam_id(id);
            *query.borrow_mut() = text;
            filter.changed(FilterChange::Different);

            if id < STEAMID64_BASE {
                custom.set_name("");
                custom.set_avatar_url("");
                last_resolved.set(0);
                return;
            }
            if last_resolved.get() == id {
                return;
            }
            last_resolved.set(id);
            custom.set_name("");
            custom.set_avatar_url("");

            let name_handle =
                spawn_blocking(move || GetUserPersonaName { steam_id64: id }.request());
            let avatar_handle = spawn_blocking(move || GetUserAvatar { steam_id64: id }.request());
            MainContext::default().spawn_local(clone!(
                #[strong]
                custom,
                async move {
                    if let Ok(Some(name)) = name_handle.await.expect("spawn_blocking task panicked")
                        && custom.steam_id() == id
                    {
                        custom.set_name(name);
                    }
                    // ShimmerImage loads from a URL, so encode the native RGBA as a
                    // data: URL and point the row's avatar-url at it.
                    if let Ok(Some(img)) =
                        avatar_handle.await.expect("spawn_blocking task panicked")
                        && custom.steam_id() == id
                        && let Some(url) = rgba_to_data_url(&img.rgba, img.width, img.height)
                    {
                        custom.set_avatar_url(url);
                    }
                }
            ));
        }
    ));

    let scroller = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .has_frame(true)
        .child(&list_view)
        .build();
    let search_row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    search_row.append(&search);
    search_row.append(&spinner);

    let content = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    // Bottom row: clear the currently-selected user (only enabled if there is one).
    let clear_button = Button::builder()
        .label(tr("Clear selected user").as_str())
        .halign(Align::End)
        .sensitive(has_selection)
        .build();
    clear_button.connect_clicked(clone!(
        #[weak]
        window,
        move |_| {
            on_clear();
            window.close();
        }
    ));

    content.append(&search_row);
    content.append(&banner);
    content.append(&scroller);
    content.append(&clear_button);

    window.set_child(Some(&content));
    window.present();
    search.grab_focus();
}

/// Encode a native RGBA avatar as an in-memory `data:image/png;base64,…` URL, so
/// the URL-based row factory can load it like any other avatar without leaving a
/// temp file behind.
fn rgba_to_data_url(rgba: &[u8], width: u32, height: u32) -> Option<String> {
    let bytes = glib::Bytes::from(rgba);
    let texture = MemoryTexture::new(
        width as i32,
        height as i32,
        MemoryFormat::R8g8b8a8,
        &bytes,
        (width as usize) * 4,
    );
    let b64 = glib::base64_encode(&texture.save_to_png_bytes());
    Some(format!("data:image/png;base64,{b64}"))
}
