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

mod config_popover;
mod copy_controls;
mod copy_mode;
mod header;
mod row_factory;

use crate::dev_println;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::mode_state::{GUnlockModeState, MODE_AUTOCOMMIT, MODE_DEFERRED};
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::unlock_queue::{UnlockQueue, resolve_target_count};
use crate::gui_frontend::unlock_scheduler::{
    SPACING_EVEN, SPACING_RANDOM, compute_unlock_times_ms, run_timed_unlock, unlock_all_immediately,
};
use crate::utils::format::format_achievement_progress;
use config_popover::create_config_popover;
use copy_controls::create_copy_controls;
use copy_mode::install_copy_mode;
use gtk::gio::ListStore;
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Box, Button, Frame, Label, ListView, NoSelection, Orientation, ScrolledWindow,
    SignalListItemFactory, SpinButton, Stack, ToggleButton, glib,
};
use header::create_header;
use row_factory::install_row_factory;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub(super) fn update_queue_label(label: &Label, queue: &UnlockQueue) {
    let n = queue.len();
    label.set_label(&match n {
        0 => tr("No achievements staged").to_string(),
        n => tr("{count} staged").replace("{count}", &n.to_string()),
    });
}

pub(super) fn update_start_sensitive(
    start_button: &Button,
    queue: &UnlockQueue,
    unlocked: usize,
    total: u32,
) {
    let all_done = unlocked == total as usize;
    start_button.set_sensitive(!all_done && !queue.is_empty());
}

fn update_autofill_sensitive(
    auto_fill_button: &Button,
    count_spin: &SpinButton,
    percent_spin: &SpinButton,
    unit_percent: &ToggleButton,
    raw_model: &ListStore,
    unlocked: usize,
) {
    let total = raw_model.n_items() as usize;
    let unit = if unit_percent.is_active() {
        "percent"
    } else {
        "count"
    };
    let to_add = resolve_target_count(
        unit,
        count_spin.value_as_int(),
        percent_spin.value(),
        total,
        unlocked,
    );
    if to_add == 0 {
        auto_fill_button.set_sensitive(false);
        auto_fill_button.set_tooltip_text(Some(
            &tr("Already at or above target ({progress})")
                .replace("{progress}", &format_achievement_progress(unlocked, total)),
        ));
    } else {
        auto_fill_button.set_sensitive(true);
        auto_fill_button.set_tooltip_text(Some(
            &tr("Auto-fill {count} achievement(s) ({progress})")
                .replace("{count}", &to_add.to_string())
                .replace("{progress}", &format_achievement_progress(unlocked, total)),
        ));
    }
}

#[inline]
pub fn create_achievements_manual_view(
    app_id: &Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: &Rc<Cell<usize>>,
    filtered_model: &NoSelection,
    raw_model: &ListStore,
    timed_raw_model: &ListStore,
    achievement_views_stack: &Stack,
    app_achievement_count_value: &Label,
    application: &MainApplication,
) -> (Frame, Arc<AtomicBool>) {
    let settings = get_settings();

    let mode_state = Rc::new(GUnlockModeState::default());
    let initial_mode = settings.string("unlock-mode").to_string();
    mode_state.set_mode(if initial_mode == MODE_DEFERRED {
        MODE_DEFERRED
    } else {
        MODE_AUTOCOMMIT
    });
    settings.bind("unlock-mode", &*mode_state, "mode").build();

    // Stage mode (manual clicks) and copy mode (a friend's cadence) each own a
    // queue; only the active mode's positions are rendered onto the rows, so the
    // two never bleed into each other when switching modes.
    let queue = UnlockQueue::new();
    let copy_queue = UnlockQueue::new();
    let cancelled_task = Arc::new(AtomicBool::new(true));

    let config = create_config_popover(&settings);
    let copy = create_copy_controls(&settings);
    let header = create_header(&mode_state, &config.popover, &copy);

    update_queue_label(&header.queue_label, &queue);
    header.start_button.set_sensitive(false);

    let update_autofill: Rc<dyn Fn()> = {
        let auto_fill_button = header.auto_fill_button.clone();
        let count_spin = config.count_spin.clone();
        let percent_spin = config.percent_spin.clone();
        let unit_percent = config.unit_percent.clone();
        let raw_model_inner = raw_model.clone();
        let unlocked_count = app_unlocked_achievements_count.clone();
        Rc::new(move || {
            update_autofill_sensitive(
                &auto_fill_button,
                &count_spin,
                &percent_spin,
                &unit_percent,
                &raw_model_inner,
                unlocked_count.get(),
            )
        })
    };
    update_autofill();

    {
        let f = Rc::clone(&update_autofill);
        config.count_spin.connect_value_notify(move |_| f());
    }
    {
        let f = Rc::clone(&update_autofill);
        config.percent_spin.connect_value_notify(move |_| f());
    }
    {
        let f = Rc::clone(&update_autofill);
        config.unit_percent.connect_toggled(move |_| f());
    }

    // Wipe the queue whenever the raw model is reset (game change, refresh, etc.).
    raw_model.connect_items_changed(clone!(
        #[strong]
        queue,
        #[weak(rename_to = queue_label)]
        header.queue_label,
        #[weak(rename_to = start_button)]
        header.start_button,
        #[strong]
        update_autofill,
        move |model, _pos, removed, _added| {
            if removed > 0 {
                queue.clear(model);
                update_queue_label(&queue_label, &queue);
                start_button.set_sensitive(false);
            }
            update_autofill();
        }
    ));

    // On a mode switch, hide both queues' positions then render only the active
    // mode's. The copy-timing queue is (re)rendered by its own handler below; here
    // we restore the staged queue when returning to deferred mode.
    mode_state.connect_mode_notify(clone!(
        #[strong]
        queue,
        #[strong]
        copy_queue,
        #[strong]
        app_unlocked_achievements_count,
        #[weak(rename_to = raw_model)]
        raw_model,
        #[weak(rename_to = queue_label)]
        header.queue_label,
        #[weak(rename_to = start_button)]
        header.start_button,
        move |state| {
            queue.hide(&raw_model);
            copy_queue.hide(&raw_model);
            if state.mode() == MODE_DEFERRED {
                queue.render(&raw_model);
                update_queue_label(&queue_label, &queue);
                update_start_sensitive(
                    &start_button,
                    &queue,
                    app_unlocked_achievements_count.get(),
                    raw_model.n_items(),
                );
            } else {
                start_button.set_sensitive(false);
            }
        }
    ));

    header.auto_fill_button.connect_clicked(clone!(
        #[strong]
        queue,
        #[strong]
        app_unlocked_achievements_count,
        #[weak(rename_to = raw_model)]
        raw_model,
        #[weak(rename_to = unit_percent)]
        config.unit_percent,
        #[weak(rename_to = count_spin)]
        config.count_spin,
        #[weak(rename_to = percent_spin)]
        config.percent_spin,
        #[weak(rename_to = queue_label)]
        header.queue_label,
        #[weak(rename_to = start_button)]
        header.start_button,
        move |_| {
            let total = raw_model.n_items() as usize;
            let unlocked = app_unlocked_achievements_count.get();
            let unit = if unit_percent.is_active() {
                "percent"
            } else {
                "count"
            };
            let to_add = resolve_target_count(
                unit,
                count_spin.value_as_int(),
                percent_spin.value(),
                total,
                unlocked,
            );
            if to_add == 0 {
                return;
            }
            queue.auto_fill(&raw_model, to_add);
            update_queue_label(&queue_label, &queue);
            update_start_sensitive(&start_button, &queue, unlocked, raw_model.n_items());
        }
    ));

    header.start_button.connect_clicked(clone!(
        #[strong]
        queue,
        #[strong]
        app_id,
        #[strong]
        cancelled_task,
        #[strong]
        timed_raw_model,
        #[weak]
        application,
        #[weak(rename_to = raw_model)]
        raw_model,
        #[weak(rename_to = duration_spin)]
        config.duration_spin,
        #[weak(rename_to = spacing_random)]
        config.spacing_random,
        #[weak(rename_to = achievement_views_stack)]
        achievement_views_stack,
        move |_| {
            let ids = queue.snapshot();
            if ids.is_empty() {
                return;
            }

            let achievements = resolve_queue_to_objects(&raw_model, &ids);
            let app_id_val = match app_id.get() {
                Some(v) => v,
                None => return,
            };

            let desired_minutes = duration_spin.value_as_int().max(0) as u64;
            if desired_minutes == 0 {
                dev_println!(
                    "CLIENT",
                    "Instant unlock of {} achievements",
                    achievements.len()
                );
                unlock_all_immediately(app_id_val, &achievements);
                application.activate_action("refresh_achievements_list", None);
                return;
            }

            let spacing = if spacing_random.is_active() {
                SPACING_RANDOM
            } else {
                SPACING_EVEN
            };
            let total_ms = desired_minutes * 60 * 1000;
            let times_ms = compute_unlock_times_ms(achievements.len(), total_ms, spacing);

            cancelled_task.store(false, std::sync::atomic::Ordering::Relaxed);

            MainContext::default().spawn_local(clone!(
                #[strong]
                timed_raw_model,
                #[strong]
                cancelled_task,
                async move {
                    run_timed_unlock(
                        app_id_val,
                        achievements,
                        times_ms,
                        timed_raw_model,
                        cancelled_task,
                    )
                    .await;
                }
            ));

            achievement_views_stack.set_visible_child_name("automatic");
        }
    ));

    install_copy_mode(
        &copy,
        &copy_queue,
        &settings,
        &mode_state,
        app_id,
        raw_model,
        timed_raw_model,
        &cancelled_task,
        achievement_views_stack,
        application,
    );

    let achievements_list_factory = SignalListItemFactory::new();
    install_row_factory(
        &achievements_list_factory,
        &mode_state,
        &queue,
        app_id,
        app_unlocked_achievements_count,
        raw_model,
        app_achievement_count_value,
        &header.start_button,
        &header.queue_label,
        &cancelled_task,
        &update_autofill,
    );

    let app_achievements_list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(filtered_model)
        .factory(&achievements_list_factory)
        .build();
    let app_achievements_scrolled_window = ScrolledWindow::builder()
        .child(&app_achievements_list_view)
        .vexpand(true)
        .build();

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&header.container);
    vbox.append(&app_achievements_scrolled_window);

    let frame = Frame::builder()
        .margin_end(15)
        .margin_start(15)
        .margin_top(15)
        .margin_bottom(15)
        .child(&vbox)
        .build();

    (frame, cancelled_task)
}

pub(super) fn resolve_queue_to_objects(
    raw_model: &ListStore,
    ids: &[String],
) -> Vec<GAchievementObject> {
    use std::collections::HashMap;
    let mut by_id: HashMap<String, GAchievementObject> = HashMap::new();
    for obj in raw_model.into_iter().flatten() {
        let ach = obj
            .downcast::<GAchievementObject>()
            .expect("Not a GAchievementObject");
        by_id.insert(ach.id(), ach);
    }
    ids.iter().filter_map(|id| by_id.remove(id)).collect()
}
