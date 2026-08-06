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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

glib::wrapper! {
    pub struct GSteamAppObject(ObjectSubclass<imp::GSteamAppObject>);
}

/// The 460x215 banner, under every name Steam has filed it as: current clients
/// write `library_header.jpg` into a hashed subdirectory, older ones wrote
/// `header.jpg` straight into the app directory.
const BANNER_NAMES: [&str; 2] = ["header.jpg", "library_header.jpg"];

#[derive(Default)]
struct LocalBannerIndex {
    prefix: Option<String>,
    paths: HashMap<u32, PathBuf>,
}

thread_local! {
    static LOCAL_BANNER_INDEX: RefCell<Option<LocalBannerIndex>> = const { RefCell::new(None) };
}

fn find_local_banner(app_dir: &Path) -> Option<PathBuf> {
    for name in BANNER_NAMES {
        let path = app_dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }

    for entry in std::fs::read_dir(app_dir).ok()?.flatten() {
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let sub_dir = entry.path();
        for name in BANNER_NAMES {
            let path = sub_dir.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

fn banner_url(path: &Path) -> Option<String> {
    Some(format!("file://{}", path.to_str()?))
}

fn build_local_banner_index() -> LocalBannerIndex {
    let prefix = SteamLocator::global()
        .read()
        .unwrap()
        .get_local_app_banner_file_prefix_cached();

    let mut paths = HashMap::new();
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
            if let Some(path) = find_local_banner(&entry.path()) {
                paths.insert(app_id, path);
            }
        }
    }

    LocalBannerIndex { prefix, paths }
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
            None => return (None, None),
        };
        (index.prefix.clone(), index.paths.get(&app_id).cloned())
    });

    if let Some(path) = cache_hit {
        return banner_url(&path);
    }

    let path = find_local_banner(&Path::new(&prefix?).join(app_id.to_string()))?;
    let url = banner_url(&path)?;

    LOCAL_BANNER_INDEX.with(|cell| {
        if let Some(index) = cell.borrow_mut().as_mut() {
            index.paths.insert(app_id, path);
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
            .property("app_name", app.app_name)
            .property("developer", app.developer)
            .property("image_url", image_url)
            .property("metacritic_score", app.metacritic_score.unwrap_or(u8::MAX))
            .property("app_type", format!("{:?}", app.app_type))
            .property("can_start_idling", true)
            .build();

        let imp = obj.imp();
        imp.is_junk.set(is_junk);
        imp.app_id.set(app.app_id);
        imp.playtime_minutes.set(app.playtime_minutes.unwrap_or(0));
        imp.last_played.set(app.last_played.unwrap_or(0));
        let _ = imp.lowercase_name.set(lowercase_name);
        obj.set_counts(
            app.achievement_count.unwrap_or(0),
            app.unlocked_achievement_count.unwrap_or(0),
            achievements_loaded,
        );
        obj
    }

    /// The counts and the two orders derived from them, written together: the
    /// sorter reads those orders on every comparison, and reaching them back
    /// through the properties costs 132 ns a field.
    pub fn set_counts(&self, total: u32, unlocked: u32, loaded: bool) {
        self.set_achievement_count(total);
        self.set_unlocked_achievement_count(unlocked);
        self.set_achievements_loaded(loaded);

        let imp = self.imp();
        imp.completion.set(if loaded && total > 0 {
            f64::from(unlocked) / f64::from(total)
        } else {
            -1.0
        });
        let remaining = total.saturating_sub(unlocked);
        imp.remaining.set(if loaded && remaining > 0 {
            remaining
        } else {
            u32::MAX
        });
    }

    pub fn completion(&self) -> f64 {
        self.imp().completion.get()
    }

    pub fn remaining(&self) -> u32 {
        self.imp().remaining.get()
    }

    /// `new` reaches for the Steam install to find the banner, and `app_id` is
    /// not a property, so a test cannot build one of these any other way.
    #[cfg(test)]
    pub fn with_app_id(app_id: u32) -> Self {
        let obj: Self = Object::builder().build();
        obj.imp().app_id.set(app_id);
        obj.set_counts(0, 0, false);
        obj
    }

    pub fn app_id(&self) -> u32 {
        self.imp().app_id.get()
    }

    pub fn playtime_minutes(&self) -> u32 {
        self.imp().playtime_minutes.get()
    }

    pub fn last_played(&self) -> u64 {
        self.imp().last_played.get()
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

        // Kept out of the property system: a derived getter round-trips through
        // a `GValue`, which the sorter would pay on every comparison.
        pub(super) app_id: Cell<u32>,
        pub(super) playtime_minutes: Cell<u32>,
        pub(super) last_played: Cell<u64>,
        pub(super) is_junk: Cell<bool>,
        pub(super) lowercase_name: OnceCell<Rc<String>>,

        pub(super) completion: Cell<f64>,
        pub(super) remaining: Cell<u32>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(total: u32, unlocked: u32, loaded: bool) -> (f64, u32) {
        let app = GSteamAppObject::with_app_id(1);
        app.set_counts(total, unlocked, loaded);
        (app.completion(), app.remaining())
    }

    #[test]
    fn an_app_still_loading_sorts_last_in_both_orders() {
        assert_eq!(keys(20, 5, false), (-1.0, u32::MAX));
    }

    #[test]
    fn an_app_without_achievements_sorts_last_in_both_orders() {
        assert_eq!(keys(0, 0, true), (-1.0, u32::MAX));
    }

    #[test]
    fn a_finished_app_is_fully_complete_but_has_nothing_left() {
        assert_eq!(keys(20, 20, true), (1.0, u32::MAX));
    }

    #[test]
    fn an_app_in_progress_carries_both_figures() {
        assert_eq!(keys(91, 90, true), (90.0 / 91.0, 1));
        assert_eq!(keys(20, 0, true), (0.0, 20));
    }
}
