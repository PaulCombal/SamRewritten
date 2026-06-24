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
//! of friends (avatar + name + this game's achieved/total count). Only visible
//! rows build widgets / load avatars / fetch counts, so a large friends list stays
//! cheap. Selecting a user is only possible by clicking a result; a pasted
//! SteamID64 surfaces as its own clickable result.

use crate::backend::user_unlock_times::{Friend, STEAMID64_BASE};
use crate::gui_frontend::gobjects::friend::GFriendObject;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{
    GetFriendAchievementCount, GetUserAvatar, GetUserPersonaName, Request,
};
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
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

/// The row's avatar widget (first child of the row's hbox).
fn row_avatar(li: &ListItem) -> Option<ShimmerImage> {
    li.child()
        .and_then(|child| child.first_child())
        .and_then(|w| w.downcast::<ShimmerImage>().ok())
}

/// The row's trailing achievement-count label (last child of the row's hbox).
fn row_count_label(li: &ListItem) -> Option<Label> {
    li.child()
        .and_then(|child| child.last_child())
        .and_then(|w| w.downcast::<Label>().ok())
}

/// Resolved per-game count for a friend, cached for the picker's lifetime so a
/// recycled row shows instantly and nobody is queried twice.
#[derive(Clone)]
enum CountState {
    Ready(u32, u32),
    Private,
    /// Queried and failed (or panicked); rendered blank, not retried this session.
    Failed,
}

/// Holds a weak handle to the count coordinator so it can re-invoke itself after
/// each completion without a strong self-reference cycle.
type PumpHolder = Rc<RefCell<Option<std::rc::Weak<dyn Fn()>>>>;

fn set_count_label(count: &Label, state: &CountState) {
    match state {
        CountState::Ready(achieved, total) => count.set_label(&format!("{achieved} / {total}")),
        CountState::Private => count.set_label(&tr("Private")),
        CountState::Failed => count.set_label(""),
    }
}

/// Open the modal and run `on_select` for the clicked friend. `on_select` loads
/// the friend's data (it returns a future) while the picker stays open with a
/// loading state; on success the window closes, on error a banner is shown so the
/// user can pick someone else. `on_clear` drops the current selection (its button
/// is enabled only when `has_selection`).
pub fn open_friend_picker<Fut>(
    parent: Option<&Window>,
    friends: Vec<Friend>,
    app_id: u32,
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
        let text = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .build();
        text.append(&name);
        text.append(&id);
        // Trailing "achieved / total" hint for the selected game (filled on bind).
        let count = Label::builder()
            .halign(Align::End)
            .valign(Align::Center)
            .css_classes(["dim-label", "numeric"])
            .build();
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
        hbox.append(&count);
        li.set_child(Some(&hbox));

        // Avatars load on bind (see connect_bind below), not from a property.

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

    // Per-row loading state. Avatars and counts both ride the single serialized
    // orchestrator IPC, so counts run through a one-at-a-time coordinator that
    // always picks the *topmost currently-visible* unresolved row — which makes
    // ordering follow the live filter/scroll instead of bind order. Avatars stay
    // fire-and-forget (they're quick) and the coordinator waits until none are in
    // flight before starting a count, so faces fill in first.
    let count_cache: Rc<RefCell<HashMap<u64, CountState>>> = Rc::new(RefCell::new(HashMap::new()));
    let bound: Rc<RefCell<Vec<glib::WeakRef<ListItem>>>> = Rc::new(RefCell::new(Vec::new()));
    let count_busy = Rc::new(Cell::new(false));
    let avatars_in_flight = Rc::new(Cell::new(0usize));
    // Lets the count coordinator re-invoke itself after each completion without a
    // strong self-reference cycle (which would leak it for the window's lifetime).
    let pump_holder: PumpHolder = Rc::new(RefCell::new(None));

    let pump: Rc<dyn Fn()> = {
        let bound = bound.clone();
        let count_cache = count_cache.clone();
        let count_busy = count_busy.clone();
        let avatars_in_flight = avatars_in_flight.clone();
        let pump_holder = pump_holder.clone();
        Rc::new(move || {
            if count_busy.get() || avatars_in_flight.get() > 0 {
                return;
            }
            // Topmost (lowest model position) visible row with no count yet.
            bound.borrow_mut().retain(|w| w.upgrade().is_some());
            let mut best: Option<(u32, u64)> = None;
            for w in bound.borrow().iter() {
                let Some(li) = w.upgrade() else { continue };
                let Some(item) = li.item().and_downcast::<GFriendObject>() else {
                    continue;
                };
                if item.is_custom() {
                    continue;
                }
                let sid = item.steam_id();
                if sid == 0 || count_cache.borrow().contains_key(&sid) {
                    continue;
                }
                let pos = li.position();
                if best.is_none_or(|(bp, _)| pos < bp) {
                    best = Some((pos, sid));
                }
            }
            let Some((_, sid)) = best else { return };

            count_busy.set(true);
            let handle = spawn_blocking(move || {
                GetFriendAchievementCount {
                    app_id,
                    steam_id64: sid,
                }
                .request()
            });
            let count_cache = count_cache.clone();
            let count_busy = count_busy.clone();
            let bound = bound.clone();
            let pump_holder = pump_holder.clone();
            MainContext::default().spawn_local(async move {
                let result = handle.await;
                count_busy.set(false);
                let state = match result {
                    Ok(Ok((achieved, total))) => CountState::Ready(achieved, total),
                    Ok(Err(SamError::ProfilePrivate)) => CountState::Private,
                    _ => CountState::Failed,
                };
                count_cache.borrow_mut().insert(sid, state.clone());
                for w in bound.borrow().iter() {
                    if let Some(li) = w.upgrade()
                        && li
                            .item()
                            .and_downcast::<GFriendObject>()
                            .map(|f| f.steam_id())
                            == Some(sid)
                        && let Some(count) = row_count_label(&li)
                    {
                        set_count_label(&count, &state);
                    }
                }
                if let Some(p) = pump_holder.borrow().clone().and_then(|w| w.upgrade()) {
                    p();
                }
            });
        })
    };
    *pump_holder.borrow_mut() = Some(Rc::downgrade(&pump));

    // Avatars aren't in the friend list payload, so fetch each visible row's
    // avatar natively (RGBA) on bind; clear it on unbind so a recycled row never
    // briefly shows the previous friend's face. Counts are left to the coordinator.
    factory.connect_bind({
        let bound = bound.clone();
        let count_cache = count_cache.clone();
        let avatars_in_flight = avatars_in_flight.clone();
        let pump = pump.clone();
        move |_, list_item| {
            let li = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            if let Some(avatar) = row_avatar(li) {
                avatar.reset();
            }
            if let Some(count) = row_count_label(li) {
                count.set_label("");
            }
            // Track this realized (visible) row so the coordinator can find it.
            bound.borrow_mut().push(li.downgrade());

            let Some(item) = li.item().and_downcast::<GFriendObject>() else {
                return;
            };
            // The custom (paste-a-SteamID) row has no friend avatar/count to show.
            if item.is_custom() {
                return;
            }
            let steam_id64 = item.steam_id();
            if steam_id64 == 0 {
                return;
            }

            // A cached count renders instantly; otherwise show a pending hint and
            // let the coordinator pick it up in visible order.
            match count_cache.borrow().get(&steam_id64) {
                Some(state) => {
                    if let Some(count) = row_count_label(li) {
                        set_count_label(&count, state);
                    }
                }
                None => {
                    if let Some(count) = row_count_label(li) {
                        count.set_label("…");
                    }
                }
            }

            avatars_in_flight.set(avatars_in_flight.get() + 1);
            let handle = spawn_blocking(move || GetUserAvatar { steam_id64 }.request());
            let li_weak = li.downgrade();
            let avatars_in_flight = avatars_in_flight.clone();
            let pump = pump.clone();
            MainContext::default().spawn_local(async move {
                let result = handle.await;
                if let Ok(res) = result
                    && let Some(li) = li_weak.upgrade()
                    // The row may have been recycled to another friend while waiting.
                    && li.item().and_downcast::<GFriendObject>().map(|f| f.steam_id())
                        == Some(steam_id64)
                    && let (Some(avatar), Ok(Some(img))) = (row_avatar(&li), res)
                {
                    avatar.set_rgba(img.width as i32, img.height as i32, &img.rgba);
                }
                avatars_in_flight.set(avatars_in_flight.get().saturating_sub(1));
                if avatars_in_flight.get() == 0 {
                    pump();
                }
            });
        }
    });
    factory.connect_unbind({
        let bound = bound.clone();
        move |_, list_item| {
            let Some(li) = list_item.downcast_ref::<ListItem>() else {
                return;
            };
            if let Some(avatar) = row_avatar(li) {
                avatar.reset();
            }
            if let Some(count) = row_count_label(li) {
                count.set_label("");
            }
            bound
                .borrow_mut()
                .retain(|w| w.upgrade().is_some_and(|r| &r != li));
        }
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
