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

use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::gio::ListStore;
use gtk::prelude::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Default)]
pub struct UnlockQueue {
    order: RefCell<Vec<String>>,
}

pub type SharedQueue = Rc<UnlockQueue>;

impl UnlockQueue {
    pub fn new() -> SharedQueue {
        Rc::new(Self::default())
    }

    pub fn len(&self) -> usize {
        self.order.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.borrow().is_empty()
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.order.borrow().clone()
    }

    pub fn toggle(&self, achievement: &GAchievementObject, raw_model: &ListStore) -> bool {
        let id = achievement.id();
        let mut order = self.order.borrow_mut();
        if let Some(pos) = order.iter().position(|x| x == &id) {
            order.remove(pos);
            achievement.set_queue_position(0);
            renumber_from(&order, pos, raw_model);
            false
        } else {
            order.push(id);
            achievement.set_queue_position(order.len() as u32);
            true
        }
    }

    pub fn clear(&self, raw_model: &ListStore) {
        let mut order = self.order.borrow_mut();
        if order.is_empty() {
            return;
        }
        let ids: std::collections::HashSet<String> = order.drain(..).collect();
        for obj in raw_model.into_iter().flatten() {
            let ach = obj
                .downcast::<GAchievementObject>()
                .expect("Not a GAchievementObject");
            if ids.contains(&ach.id()) {
                ach.set_queue_position(0);
            }
        }
    }

    /// Replace the queue with `ordered_ids` in the given order, setting each
    /// matching achievement's queue position. Used by copy-timing mode, where the
    /// order comes from a friend's timeline rather than the user's clicks. Ids not
    /// present in the model are skipped; callers pre-filter achieved/protected.
    pub fn set_order(&self, raw_model: &ListStore, ordered_ids: &[String]) {
        self.clear(raw_model);
        let mut by_id: HashMap<String, GAchievementObject> = HashMap::new();
        for obj in raw_model.into_iter().flatten() {
            if let Ok(ach) = obj.downcast::<GAchievementObject>() {
                by_id.insert(ach.id(), ach);
            }
        }
        let mut order = self.order.borrow_mut();
        for id in ordered_ids {
            if let Some(ach) = by_id.get(id) {
                order.push(id.clone());
                ach.set_queue_position(order.len() as u32);
            }
        }
    }

    /// Re-apply each queued id's 1-based position to its achievement without
    /// changing the stored order. Used to bring a queue back on screen after a
    /// mode switch hid it.
    pub fn render(&self, raw_model: &ListStore) {
        let order = self.order.borrow();
        if order.is_empty() {
            return;
        }
        let pos_by_id: HashMap<&str, u32> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), (i + 1) as u32))
            .collect();
        for obj in raw_model.into_iter().flatten() {
            let ach = obj
                .downcast::<GAchievementObject>()
                .expect("Not a GAchievementObject");
            if let Some(&pos) = pos_by_id.get(ach.id().as_str()) {
                ach.set_queue_position(pos);
            }
        }
    }

    /// Zero the on-screen positions of queued achievements while keeping the
    /// stored order, so the queue can be re-rendered later. Lets the staged and
    /// copy-timing queues coexist without bleeding into one another.
    pub fn hide(&self, raw_model: &ListStore) {
        let order = self.order.borrow();
        if order.is_empty() {
            return;
        }
        let ids: HashSet<&str> = order.iter().map(|s| s.as_str()).collect();
        for obj in raw_model.into_iter().flatten() {
            let ach = obj
                .downcast::<GAchievementObject>()
                .expect("Not a GAchievementObject");
            if ids.contains(ach.id().as_str()) {
                ach.set_queue_position(0);
            }
        }
    }

    pub fn auto_fill(&self, raw_model: &ListStore, target_count: usize) {
        self.clear(raw_model);
        if target_count == 0 {
            return;
        }

        let mut candidates: Vec<GAchievementObject> = raw_model
            .into_iter()
            .flatten()
            .filter_map(|obj| obj.downcast::<GAchievementObject>().ok())
            .filter(|ach| !ach.is_achieved() && ach.permission() == 0)
            .collect();

        candidates.sort_by(|a, b| {
            let pa = a.global_achieved_percent();
            let pb = b.global_achieved_percent();
            pb.partial_cmp(&pa).unwrap_or(Ordering::Equal)
        });

        candidates.truncate(target_count);

        let mut order = self.order.borrow_mut();
        for (i, ach) in candidates.iter().enumerate() {
            ach.set_queue_position((i + 1) as u32);
            order.push(ach.id());
        }
    }
}

fn renumber_from(order: &[String], start: usize, raw_model: &ListStore) {
    if start >= order.len() {
        return;
    }
    let mut id_to_pos: HashMap<&str, u32> = HashMap::new();
    for (i, id) in order.iter().enumerate().skip(start) {
        id_to_pos.insert(id.as_str(), (i + 1) as u32);
    }
    for obj in raw_model.into_iter().flatten() {
        let ach = obj
            .downcast::<GAchievementObject>()
            .expect("Not a GAchievementObject");
        if let Some(&new_pos) = id_to_pos.get(ach.id().as_str()) {
            ach.set_queue_position(new_pos);
        }
    }
}

pub fn resolve_target_count(
    unit: &str,
    count_value: i32,
    percent_value: f64,
    total: usize,
    already_unlocked: usize,
) -> usize {
    let target_unlocked = match unit {
        "percent" => {
            let pct = percent_value.clamp(0.0, 100.0) / 100.0;
            (pct * total as f64).ceil() as usize
        }
        _ => count_value.max(0) as usize,
    };
    target_unlocked.saturating_sub(already_unlocked)
}
