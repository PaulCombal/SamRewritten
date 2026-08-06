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

use crate::backend::app_lister::ACHIEVEMENT_COUNT_CHUNK_SIZE;
use crate::gui_frontend::app_list_view::app_index::AppIndex;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::request::{GetAchievementCounts, Request};
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::MainContext;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

// Insertion-ordered: a plain HashSet would scramble "visible first" and
// library order both.
#[derive(Default)]
struct OrderedSet {
    order: VecDeque<u32>,
    set: HashSet<u32>,
}

impl OrderedSet {
    fn insert(&mut self, id: u32) -> bool {
        if self.set.insert(id) {
            self.order.push_back(id);
            true
        } else {
            false
        }
    }

    fn remove(&mut self, id: u32) -> bool {
        if !self.set.remove(&id) {
            return false;
        }
        if let Some(pos) = self.order.iter().position(|&x| x == id) {
            self.order.remove(pos);
        }
        true
    }

    fn contains(&self, id: u32) -> bool {
        self.set.contains(&id)
    }

    fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    fn clear(&mut self) {
        self.order.clear();
        self.set.clear();
    }

    fn drain_front(&mut self, n: usize) -> Vec<u32> {
        let take = n.min(self.order.len());
        let mut out = Vec::with_capacity(take);
        for _ in 0..take {
            if let Some(id) = self.order.pop_front() {
                self.set.remove(&id);
                out.push(id);
            }
        }
        out
    }
}

type CountsAppliedHook = Rc<RefCell<Option<Box<dyn Fn()>>>>;

#[derive(Default, Clone)]
pub struct AchievementLoader {
    priority: Rc<RefCell<OrderedSet>>,
    backlog: Rc<RefCell<OrderedSet>>,
    in_flight: Rc<RefCell<HashSet<u32>>>,
    worker_running: Rc<Cell<bool>>,
    generation: Rc<Cell<u64>>,
    counts_applied: CountsAppliedHook,
    index: Rc<RefCell<Option<AppIndex>>>,
    queued_generation: Rc<Cell<Option<u64>>>,
}

impl AchievementLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Before anything else listens for `items-changed`: handlers run in
    /// connection order, and a reader ahead of this one would be served the
    /// tally from before the change.
    pub fn attach(&self, list_store: &ListStore) {
        self.index(list_store);
    }

    fn index(&self, list_store: &ListStore) -> AppIndex {
        let mut slot = self.index.borrow_mut();
        if let Some(index) = slot.as_ref()
            && index.store() == list_store
        {
            return index.clone();
        }
        let index = AppIndex::new(list_store);
        *slot = Some(index.clone());
        index
    }

    pub fn counts_progress(&self, list_store: &ListStore) -> (u32, u32) {
        self.index(list_store).progress()
    }

    pub fn on_counts_applied(&self, f: impl Fn() + 'static) {
        *self.counts_applied.borrow_mut() = Some(Box::new(f));
    }

    pub fn reset(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.priority.borrow_mut().clear();
        self.backlog.borrow_mut().clear();
        self.in_flight.borrow_mut().clear();
        self.worker_running.set(false);
    }

    pub fn queue_remaining(&self, list_store: &ListStore) {
        let generation = self.index(list_store).generation();
        let refill =
            self.backlog.borrow().is_empty() || self.queued_generation.get() != Some(generation);
        if refill {
            let mut backlog = self.backlog.borrow_mut();
            let priority = self.priority.borrow();
            let in_flight = self.in_flight.borrow();
            for i in 0..list_store.n_items() {
                let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>() else {
                    continue;
                };
                if app.achievements_loaded() || app.is_synthetic() {
                    continue;
                }
                let app_id = app.app_id();
                if priority.contains(app_id) || in_flight.contains(&app_id) {
                    continue;
                }
                backlog.insert(app_id);
            }
            self.queued_generation.set(Some(generation));
        }
        self.kick(list_store);
    }

    pub fn cancel_backlog(&self) {
        self.backlog.borrow_mut().clear();
    }

    pub fn prioritize(&self, app_id: u32) {
        if self.in_flight.borrow().contains(&app_id) {
            return;
        }
        if self.priority.borrow().contains(app_id) {
            return;
        }
        self.backlog.borrow_mut().remove(app_id);
        self.priority.borrow_mut().insert(app_id);
    }

    pub fn refresh_app(&self, app_id: u32, list_store: &ListStore) {
        self.backlog.borrow_mut().remove(app_id);
        self.priority.borrow_mut().insert(app_id);
        self.kick(list_store);
    }

    pub fn is_working(&self) -> bool {
        self.worker_running.get()
    }

    pub fn kick(&self, list_store: &ListStore) {
        if self.worker_running.get() {
            return;
        }
        if self.priority.borrow().is_empty() && self.backlog.borrow().is_empty() {
            return;
        }
        self.worker_running.set(true);
        let gen_snapshot = self.generation.get();
        let loader = self.clone();
        let index = self.index(list_store);

        MainContext::default().spawn_local(async move {
            loop {
                // A stale worker leaves the flag alone: `reset` cleared it and
                // the next `kick` has taken it, so clearing it here would let a
                // third worker start alongside the one now running.
                if loader.generation.get() != gen_snapshot {
                    return;
                }

                let chunk = loader.drain_chunk();
                if chunk.is_empty() {
                    loader.worker_running.set(false);
                    return;
                }

                loader.in_flight.borrow_mut().extend(chunk.iter().copied());

                let chunk_for_request = chunk.clone();
                let handle = spawn_blocking(move || {
                    GetAchievementCounts {
                        app_ids: chunk_for_request,
                    }
                    .request()
                });

                let result = handle.await;

                {
                    let mut in_flight = loader.in_flight.borrow_mut();
                    for id in &chunk {
                        in_flight.remove(id);
                    }
                }

                if loader.generation.get() != gen_snapshot {
                    return;
                }

                let counts = match result {
                    Ok(Ok(c)) => c,
                    Ok(Err(e)) => {
                        eprintln!("[CLIENT] GetAchievementCounts failed: {e}");
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[CLIENT] GetAchievementCounts join failed: {e:?}");
                        continue;
                    }
                };

                apply_counts(&index, &chunk, counts);

                if let Some(on_applied) = loader.counts_applied.borrow().as_ref() {
                    on_applied();
                }
            }
        });
    }

    fn drain_chunk(&self) -> Vec<u32> {
        let mut chunk = self
            .priority
            .borrow_mut()
            .drain_front(ACHIEVEMENT_COUNT_CHUNK_SIZE);
        if chunk.len() < ACHIEVEMENT_COUNT_CHUNK_SIZE {
            let remaining = ACHIEVEMENT_COUNT_CHUNK_SIZE - chunk.len();
            chunk.extend(self.backlog.borrow_mut().drain_front(remaining));
        }
        chunk
    }
}

fn apply_counts(index: &AppIndex, chunk: &[u32], counts: Vec<(u32, u32, u32)>) {
    let response_map: HashMap<u32, (u32, u32)> = counts
        .into_iter()
        .map(|(id, total, unlocked)| (id, (total, unlocked)))
        .collect();

    for app_id in chunk {
        index.update(*app_id, |app| {
            let (total, unlocked) = response_map.get(app_id).copied().unwrap_or((0, 0));
            app.set_counts(total, unlocked, true);
        });
    }
}
