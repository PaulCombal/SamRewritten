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
use crate::gui_frontend::request::{GetAchievementCounts, Request};
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::MainContext;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

// Must match CHUNK_SIZE in backend::app_lister::fetch_achievement_counts.
const CHUNK_SIZE: usize = 50;

// Insertion-ordered set: drains in FIFO order so "visible first" prioritization
// and library order are preserved. A plain HashSet would scramble both.
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

#[derive(Default, Clone)]
pub struct AchievementLoader {
    priority: Rc<RefCell<OrderedSet>>,
    backlog: Rc<RefCell<OrderedSet>>,
    in_flight: Rc<RefCell<HashSet<u32>>>,
    worker_running: Rc<Cell<bool>>,
    generation: Rc<Cell<u64>>,
}

impl AchievementLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop all queued work and bump the generation so any in-flight worker
    /// bails out. Schemas are fetched lazily via `prioritize` as cards scroll
    /// into view; no startup batch (Steam's stats queue can't keep up).
    pub fn reset(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.priority.borrow_mut().clear();
        self.backlog.borrow_mut().clear();
        self.in_flight.borrow_mut().clear();
        self.worker_running.set(false);
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

    /// Re-fetch even if loaded or in-flight: caller has reason to believe the
    /// current counts are stale (e.g. user just edited achievements).
    pub fn refresh_app(&self, app_id: u32, list_store: &ListStore) {
        self.backlog.borrow_mut().remove(app_id);
        self.priority.borrow_mut().insert(app_id);
        self.kick(list_store);
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
        let list_store = list_store.clone();

        MainContext::default().spawn_local(async move {
            loop {
                if loader.generation.get() != gen_snapshot {
                    loader.worker_running.set(false);
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
                    loader.worker_running.set(false);
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

                apply_counts(&list_store, &chunk, counts);
            }
        });
    }

    fn drain_chunk(&self) -> Vec<u32> {
        let mut chunk = self.priority.borrow_mut().drain_front(CHUNK_SIZE);
        if chunk.len() < CHUNK_SIZE {
            let remaining = CHUNK_SIZE - chunk.len();
            chunk.extend(self.backlog.borrow_mut().drain_front(remaining));
        }
        chunk
    }
}

// Apps missing from `counts` (schema didn't load in time) are still marked
// loaded with zero counts to keep them out of the backlog.
fn apply_counts(list_store: &ListStore, chunk: &[u32], counts: Vec<(u32, u32, u32)>) {
    let mut by_id: HashMap<u32, GSteamAppObject> =
        HashMap::with_capacity(list_store.n_items() as usize);
    for i in 0..list_store.n_items() {
        if let Some(item) = list_store.item(i)
            && let Ok(app) = item.downcast::<GSteamAppObject>()
        {
            by_id.insert(app.app_id(), app);
        }
    }

    let response_map: HashMap<u32, (u32, u32)> = counts
        .into_iter()
        .map(|(id, total, unlocked)| (id, (total, unlocked)))
        .collect();

    for app_id in chunk {
        let Some(app) = by_id.get(app_id) else {
            continue;
        };
        if let Some(&(total, unlocked)) = response_map.get(app_id) {
            app.set_achievement_count(total);
            app.set_unlocked_achievement_count(unlocked);
        } else {
            app.set_achievement_count(0);
            app.set_unlocked_achievement_count(0);
        }
        app.set_achievements_loaded(true);
    }
}
