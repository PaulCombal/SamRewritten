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

use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use gtk::gio::ListStore;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

struct State {
    by_id: HashMap<u32, GSteamAppObject>,
    loaded: u32,
    total: u32,
}

/// App-id lookup over the list store, plus the `(loaded, total)` tally.
/// Recomputing either per chunk made a sweep quadratic in library size.
#[derive(Clone)]
pub struct AppIndex {
    store: ListStore,
    state: Rc<RefCell<Option<State>>>,
    generation: Rc<Cell<u64>>,
}

impl AppIndex {
    pub fn new(store: &ListStore) -> Self {
        let state: Rc<RefCell<Option<State>>> = Rc::new(RefCell::new(None));
        let generation = Rc::new(Cell::new(0u64));
        store.connect_items_changed({
            let state = Rc::clone(&state);
            let generation = Rc::clone(&generation);
            move |_, _, _, _| {
                *state.borrow_mut() = None;
                generation.set(generation.get().wrapping_add(1));
            }
        });
        Self {
            store: store.clone(),
            state,
            generation,
        }
    }

    pub fn store(&self) -> &ListStore {
        &self.store
    }

    pub fn generation(&self) -> u64 {
        self.generation.get()
    }

    /// The completion filters apply only once the two are equal: they compare
    /// apps against each other, so a half-loaded library would hide on data
    /// that is merely missing.
    pub fn progress(&self) -> (u32, u32) {
        self.ensure();
        self.state
            .borrow()
            .as_ref()
            .map_or((0, 0), |state| (state.loaded, state.total))
    }

    pub fn get(&self, app_id: u32) -> Option<GSteamAppObject> {
        self.ensure();
        self.state.borrow().as_ref()?.by_id.get(&app_id).cloned()
    }

    pub fn update(&self, app_id: u32, f: impl FnOnce(&GSteamAppObject)) {
        let Some(app) = self.get(app_id) else {
            return;
        };
        let was_loaded = app.achievements_loaded();
        f(&app);
        if was_loaded || !app.achievements_loaded() || app.is_synthetic() {
            return;
        }
        if let Some(state) = self.state.borrow_mut().as_mut() {
            state.loaded += 1;
        }
    }

    fn ensure(&self) {
        if self.state.borrow().is_some() {
            return;
        }
        let mut by_id = HashMap::with_capacity(self.store.n_items() as usize);
        let (mut loaded, mut total) = (0, 0);
        for i in 0..self.store.n_items() {
            let Some(app) = self.store.item(i).and_downcast::<GSteamAppObject>() else {
                continue;
            };
            if !app.is_synthetic() {
                total += 1;
                if app.achievements_loaded() {
                    loaded += 1;
                }
            }
            by_id.insert(app.app_id(), app);
        }
        *self.state.borrow_mut() = Some(State {
            by_id,
            loaded,
            total,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(app_id: u32, loaded: bool, synthetic: bool) -> GSteamAppObject {
        let app = GSteamAppObject::with_app_id(app_id);
        app.set_counts(0, 0, loaded);
        app.set_is_synthetic(synthetic);
        app
    }

    fn store(apps: &[GSteamAppObject]) -> ListStore {
        let store = ListStore::new::<GSteamAppObject>();
        store.extend_from_slice(apps);
        store
    }

    #[test]
    fn search_cards_are_not_part_of_the_library() {
        let store = store(&[
            app(1, false, false),
            app(2, true, false),
            app(3, false, true),
        ]);
        assert_eq!(AppIndex::new(&store).progress(), (1, 2));
    }

    #[test]
    fn a_first_load_moves_the_tally_and_a_second_one_does_not() {
        let store = store(&[app(1, false, false), app(2, false, false)]);
        let index = AppIndex::new(&store);
        assert_eq!(index.progress(), (0, 2));

        index.update(1, |app| app.set_counts(10, 4, true));
        assert_eq!(index.progress(), (1, 2));

        index.update(1, |app| app.set_counts(10, 7, true));
        assert_eq!(index.progress(), (1, 2));
    }

    #[test]
    fn a_library_refresh_is_not_answered_from_the_old_one() {
        let store = store(&[app(1, true, false)]);
        let index = AppIndex::new(&store);
        assert_eq!(index.progress(), (1, 1));
        let generation = index.generation();

        store.remove_all();
        store.extend_from_slice(&[app(4, false, false), app(5, false, false)]);

        assert_eq!(index.progress(), (0, 2));
        assert!(index.get(1).is_none());
        assert!(index.get(4).is_some());
        assert_ne!(index.generation(), generation);
    }
}
