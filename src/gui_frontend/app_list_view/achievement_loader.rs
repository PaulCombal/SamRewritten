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

    fn retain(&mut self, keep: impl Fn(u32) -> bool) {
        self.order.retain(|&id| keep(id));
        self.set.retain(|&id| keep(id));
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
type RescanStartedHook = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type SweepFinishedHook = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;

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
    force: Rc<Cell<bool>>,
    rescan_started: RescanStartedHook,
    sweep_finished: SweepFinishedHook,
    retried: Rc<RefCell<HashSet<u32>>>,
    transport_failures: Rc<Cell<u32>>,
    hold: Rc<Cell<bool>>,
    dirty: Rc<RefCell<HashMap<u32, u64>>>,
    marks: Rc<Cell<u64>>,
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

    pub fn on_rescan_started(&self, f: impl Fn() + 'static) {
        *self.rescan_started.borrow_mut() = Some(Box::new(f));
    }

    pub fn on_sweep_finished(&self, f: impl Fn(bool) + 'static) {
        *self.sweep_finished.borrow_mut() = Some(Box::new(f));
    }

    pub fn reset(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.priority.borrow_mut().clear();
        self.backlog.borrow_mut().clear();
        self.in_flight.borrow_mut().clear();
        self.worker_running.set(false);
        self.force.set(false);
        self.retried.borrow_mut().clear();
        self.transport_failures.set(0);
        self.hold.set(false);
    }

    /// The clearing is not cosmetic: the progress tally counts loaded apps, and
    /// before a rescan every one of them is loaded.
    pub fn rescan_all(&self, list_store: &ListStore) {
        self.reset();
        self.force.set(true);
        self.index(list_store).clear_loaded();
        self.queue_remaining(list_store);
        // Nothing to scan: without this `force` would stay set with no worker
        // left to clear it, and every later rescan would be refused.
        if !self.worker_running.get() {
            self.force.set(false);
            return;
        }
        if let Some(started) = self.rescan_started.borrow().as_ref() {
            started();
        }
    }

    pub fn is_rescanning(&self) -> bool {
        self.force.get()
    }

    /// Reported as an unforced finish, so a cancelled sweep is never mistaken
    /// for a scan that covered the library.
    pub fn cancel_sweep(&self, list_store: &ListStore) {
        self.reset();
        self.queue_remaining(list_store);
        if let Some(finished) = self.sweep_finished.borrow().as_ref() {
            finished(false);
        }
    }

    pub fn hold_sweep(&self) {
        self.hold.set(true);
    }

    pub fn release_sweep(&self, list_store: &ListStore) {
        if self.hold.replace(false) {
            self.kick(list_store);
        }
    }

    pub fn apply_local(&self, list_store: &ListStore, counts: &HashMap<u32, (u32, u32)>) {
        if self.force.get() {
            self.release_sweep(list_store);
            return;
        }
        let index = self.index(list_store);
        let dirty = self.dirty.borrow().clone();
        let wrote = Cell::new(false);
        let mut applied: HashSet<u32> = HashSet::with_capacity(counts.len());
        for (&app_id, &(total, unlocked)) in counts {
            if dirty.contains_key(&app_id) {
                continue;
            }
            index.update(app_id, |app| {
                if !app.achievements_loaded() {
                    app.set_counts(total, unlocked, true);
                    wrote.set(true);
                }
            });
            if wrote.replace(false) {
                applied.insert(app_id);
            }
        }
        self.priority
            .borrow_mut()
            .retain(|id| !applied.contains(&id));
        self.backlog
            .borrow_mut()
            .retain(|id| !applied.contains(&id));
        self.release_sweep(list_store);
        // The files can settle the whole library, and then no sweep ever runs.
        if applied.is_empty() {
            return;
        }
        if let Some(on_applied) = self.counts_applied.borrow().as_ref() {
            on_applied();
        }
    }

    pub fn queue_apps(&self, app_ids: &[u32], list_store: &ListStore) {
        {
            let mut backlog = self.backlog.borrow_mut();
            let priority = self.priority.borrow();
            let in_flight = self.in_flight.borrow();
            for &app_id in app_ids {
                if priority.contains(app_id) || in_flight.contains(&app_id) {
                    continue;
                }
                backlog.insert(app_id);
            }
        }
        self.kick(list_store);
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

    /// Marked dirty: called right after the app's progress was written, and
    /// Steam rewrites its own stats file whenever it feels like it.
    pub fn refresh_app(&self, app_id: u32, list_store: &ListStore) {
        self.backlog.borrow_mut().remove(app_id);
        self.priority.borrow_mut().insert(app_id);
        // Numbered, not just flagged: a second write while the first request
        // is in flight would otherwise be cleared by an answer predating it.
        self.marks.set(self.marks.get().wrapping_add(1));
        self.dirty.borrow_mut().insert(app_id, self.marks.get());
        self.kick(list_store);
    }

    pub fn is_working(&self) -> bool {
        self.worker_running.get() || self.hold.get()
    }

    pub fn kick(&self, list_store: &ListStore) {
        if self.worker_running.get() || self.hold.get() {
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

                if loader.hold.get() {
                    loader.worker_running.set(false);
                    return;
                }

                let chunk = loader.drain_chunk();
                if chunk.is_empty() {
                    loader.worker_running.set(false);
                    loader.retried.borrow_mut().clear();
                    loader.transport_failures.set(0);
                    let was_forced = loader.force.replace(false);
                    if let Some(finished) = loader.sweep_finished.borrow().as_ref() {
                        finished(was_forced);
                    }
                    return;
                }

                loader.in_flight.borrow_mut().extend(chunk.iter().copied());

                let chunk_for_request = chunk.clone();
                let marked: Vec<(u32, u64)> = {
                    let dirty = loader.dirty.borrow();
                    chunk
                        .iter()
                        .filter_map(|id| Some((*id, *dirty.get(id)?)))
                        .collect()
                };
                let force = loader.force.get() || !marked.is_empty();
                let handle = spawn_blocking(move || {
                    GetAchievementCounts {
                        app_ids: chunk_for_request,
                        force,
                    }
                    .request()
                });

                let result = handle.await;

                if loader.generation.get() != gen_snapshot {
                    return;
                }

                {
                    let mut in_flight = loader.in_flight.borrow_mut();
                    for id in &chunk {
                        in_flight.remove(id);
                    }
                }

                let counts = match result {
                    Ok(Ok(c)) => {
                        loader.transport_failures.set(0);
                        c
                    }
                    Ok(Err(e)) => {
                        eprintln!("[CLIENT] GetAchievementCounts failed: {e}");
                        if loader.transport_failed(&chunk) {
                            loader.abandon_sweep();
                            return;
                        }
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[CLIENT] GetAchievementCounts join failed: {e:?}");
                        if loader.transport_failed(&chunk) {
                            loader.abandon_sweep();
                            return;
                        }
                        continue;
                    }
                };

                {
                    let mut dirty = loader.dirty.borrow_mut();
                    for (id, mark) in &marked {
                        if dirty.get(id) == Some(mark)
                            && counts.iter().any(|(answered, _, _)| answered == id)
                        {
                            dirty.remove(id);
                        }
                    }
                }
                let unanswered = apply_counts(&index, &chunk, counts);
                if !unanswered.is_empty() {
                    let exhausted = loader.requeue_once(&unanswered);
                    let mut dirty = loader.dirty.borrow_mut();
                    for app_id in &exhausted {
                        dirty.remove(app_id);
                    }
                    drop(dirty);
                    settle_unknown(&index, &exhausted);
                }

                if let Some(on_applied) = loader.counts_applied.borrow().as_ref() {
                    on_applied();
                }
            }
        });
    }

    fn transport_failed(&self, chunk: &[u32]) -> bool {
        {
            let mut backlog = self.backlog.borrow_mut();
            let priority = self.priority.borrow();
            for app_id in chunk {
                if !priority.contains(*app_id) {
                    backlog.insert(*app_id);
                }
            }
        }
        self.transport_failures
            .set(self.transport_failures.get() + 1);
        self.transport_failures.get() >= 2
    }

    fn abandon_sweep(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.in_flight.borrow_mut().clear();
        self.worker_running.set(false);
        self.force.set(false);
        self.retried.borrow_mut().clear();
        self.transport_failures.set(0);
        if let Some(finished) = self.sweep_finished.borrow().as_ref() {
            finished(false);
        }
    }

    fn requeue_once(&self, chunk: &[u32]) -> Vec<u32> {
        let mut retried = self.retried.borrow_mut();
        let mut backlog = self.backlog.borrow_mut();
        let priority = self.priority.borrow();
        let mut exhausted = Vec::new();
        for app_id in chunk {
            // A second copy is read again unmarked, landing on the fresh one.
            if priority.contains(*app_id) {
                continue;
            }
            if retried.insert(*app_id) {
                backlog.insert(*app_id);
            } else {
                exhausted.push(*app_id);
            }
        }
        exhausted
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

/// Returns what Steam did not answer for; a zero would stick as a real count.
fn apply_counts(index: &AppIndex, chunk: &[u32], counts: Vec<(u32, u32, u32)>) -> Vec<u32> {
    let response_map: HashMap<u32, (u32, u32)> = counts
        .into_iter()
        .map(|(id, total, unlocked)| (id, (total, unlocked)))
        .collect();

    let mut unanswered = Vec::new();
    for app_id in chunk {
        let Some((total, unlocked)) = response_map.get(app_id).copied() else {
            let settled = Cell::new(true);
            index.update(*app_id, |app| settled.set(app.achievements_loaded()));
            if !settled.get() {
                unanswered.push(*app_id);
            }
            continue;
        };
        index.update(*app_id, |app| app.set_counts(total, unlocked, true));
    }
    unanswered
}

fn settle_unknown(index: &AppIndex, app_ids: &[u32]) {
    for app_id in app_ids {
        index.update(*app_id, |app| {
            if !app.achievements_loaded() {
                app.set_counts(0, 0, true);
            }
        });
    }
}
