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

use crate::backend::app_lister::{AppModel, AppModelType};
use crate::utils::steam_locator::SteamLocator;
use glib::Object;
use gtk::glib;
use gtk::subclass::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

glib::wrapper! {
    pub struct GSteamAppObject(ObjectSubclass<imp::GSteamAppObject>);
}

#[derive(Default)]
struct LocalBannerIndex {
    prefix: Option<String>,
    app_ids: HashSet<u32>,
}

thread_local! {
    static LOCAL_BANNER_INDEX: RefCell<Option<LocalBannerIndex>> = const { RefCell::new(None) };
}

fn build_local_banner_index() -> LocalBannerIndex {
    let prefix = SteamLocator::global()
        .read()
        .unwrap()
        .get_local_app_banner_file_prefix_cached();

    let mut app_ids = HashSet::new();
    if let Some(ref prefix) = prefix
        && let Ok(entries) = std::fs::read_dir(prefix)
    {
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Ok(app_id) = name.parse::<u32>() else {
                continue;
            };
            let mut header_path = entry.path();
            header_path.push("header.jpg");
            if header_path.exists() {
                app_ids.insert(app_id);
            }
        }
    }

    LocalBannerIndex { prefix, app_ids }
}

fn local_banner_url(app_id: u32) -> Option<String> {
    let needs_build = LOCAL_BANNER_INDEX.with(|cell| cell.borrow().is_none());
    if needs_build {
        let index = build_local_banner_index();
        LOCAL_BANNER_INDEX.with(|cell| *cell.borrow_mut() = Some(index));
    }
    let (prefix, cache_hit) = LOCAL_BANNER_INDEX.with(|cell| {
        let borrow = cell.borrow();
        let index = match borrow.as_ref() {
            Some(i) => i,
            None => return (None, false),
        };
        (index.prefix.clone(), index.app_ids.contains(&app_id))
    });
    let prefix = prefix?;
    let url = format!("file://{}{}/header.jpg", prefix, app_id);

    if cache_hit {
        return Some(url);
    }

    let path = format!("{}{}/header.jpg", prefix, app_id);
    if !std::path::Path::new(&path).exists() {
        return None;
    }

    LOCAL_BANNER_INDEX.with(|cell| {
        if let Some(index) = cell.borrow_mut().as_mut() {
            index.app_ids.insert(app_id);
        }
    });
    Some(url)
}

impl GSteamAppObject {
    pub fn rebuild_local_banner_index() {
        let index = build_local_banner_index();
        LOCAL_BANNER_INDEX.with(|cell| *cell.borrow_mut() = Some(index));
    }

    pub fn new(app: AppModel) -> Self {
        // We are client code. If a local image is already present, do not use the remote one.
        let image_url = local_banner_url(app.app_id).or(app.image_url);

        let is_junk = matches!(app.app_type, AppModelType::Junk);
        let lowercase_name = Rc::new(app.app_name.to_lowercase());

        let achievements_loaded = app.achievement_count.is_some();
        let obj: Self = Object::builder()
            .property("app_id", app.app_id)
            .property("app_name", app.app_name)
            .property("developer", app.developer)
            .property("image_url", image_url)
            .property("metacritic_score", app.metacritic_score.unwrap_or(u8::MAX))
            .property("app_type", format!("{:?}", app.app_type))
            .property("playtime_minutes", app.playtime_minutes.unwrap_or(0))
            .property("last_played", app.last_played.unwrap_or(0))
            .property("can_start_idling", true)
            .property("achievement_count", app.achievement_count.unwrap_or(0))
            .property(
                "unlocked_achievement_count",
                app.unlocked_achievement_count.unwrap_or(0),
            )
            .property("achievements_loaded", achievements_loaded)
            .build();

        let imp = obj.imp();
        imp.is_junk.set(is_junk);
        let _ = imp.lowercase_name.set(lowercase_name);
        obj
    }

    pub fn is_junk(&self) -> bool {
        self.imp().is_junk.get()
    }

    pub fn lowercase_name(&self) -> Rc<String> {
        self.imp()
            .lowercase_name
            .get()
            .expect("lowercase_name not initialized")
            .clone()
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, OnceCell, RefCell};
    use std::rc::Rc;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GSteamAppObject)]
    pub struct GSteamAppObject {
        #[property(get, set)]
        app_id: Cell<u32>,

        #[property(get, set)]
        app_name: RefCell<String>,

        #[property(get, set)]
        developer: RefCell<String>,

        #[property(get, set)]
        metacritic_score: Cell<u8>,

        #[property(get, set)]
        image_url: RefCell<Option<String>>,

        #[property(get, set)]
        app_type: RefCell<String>,

        #[property(get, set)]
        playtime_minutes: Cell<u32>,

        #[property(get, set)]
        last_played: Cell<u64>,

        #[property(get, set)]
        is_idling: Cell<bool>,

        // True for the placeholder card shown when the user types an AppId into the search bar
        #[property(get, set)]
        is_synthetic: Cell<bool>,

        #[property(get, set)]
        can_start_idling: Cell<bool>,

        #[property(get, set)]
        achievement_count: Cell<u32>,

        #[property(get, set)]
        unlocked_achievement_count: Cell<u32>,

        #[property(get, set)]
        achievements_loaded: Cell<bool>,

        // Cached for hot-path filter/sort reads (not GObject properties).
        pub(super) is_junk: Cell<bool>,
        pub(super) lowercase_name: OnceCell<Rc<String>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for GSteamAppObject {
        const NAME: &'static str = "GSteamAppObject";
        type Type = super::GSteamAppObject;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for GSteamAppObject {}
}
