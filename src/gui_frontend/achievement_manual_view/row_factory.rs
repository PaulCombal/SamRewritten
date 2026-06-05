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

use super::{update_queue_label, update_start_sensitive};
use crate::dev_println;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::mode_state::GUnlockModeState;
use crate::gui_frontend::request::{Request, SetAchievement};
use crate::gui_frontend::unlock_queue::UnlockQueue;
use crate::gui_frontend::widgets::achievement_row::AchievementRow;
use crate::utils::format::format_achievement_progress;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::{self, MainContext, clone};
use gtk::prelude::*;
use gtk::{Button, Label, ListItem, SignalListItemFactory};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[allow(clippy::too_many_arguments)]
pub(super) fn install_row_factory(
    factory: &SignalListItemFactory,
    mode_state: &Rc<GUnlockModeState>,
    queue: &Rc<UnlockQueue>,
    app_id: &Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: &Rc<Cell<usize>>,
    raw_model: &ListStore,
    app_achievement_count_value: &Label,
    start_button: &Button,
    queue_label: &Label,
    cancelled_task: &Arc<AtomicBool>,
    update_autofill: &Rc<dyn Fn()>,
) {
    factory.connect_setup(clone!(
        #[strong]
        mode_state,
        #[strong]
        queue,
        #[strong]
        app_id,
        #[strong]
        app_unlocked_achievements_count,
        #[strong]
        cancelled_task,
        #[strong]
        update_autofill,
        #[weak]
        raw_model,
        #[weak]
        app_achievement_count_value,
        #[weak]
        start_button,
        #[weak]
        queue_label,
        move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");

            let row = AchievementRow::new();
            list_item.set_child(Some(&row));
            row.bind_display(list_item, &mode_state);

            // Autocommit switch handler — instant unlock on toggle.
            row.switch().connect_state_notify(clone!(
                #[weak]
                list_item,
                #[strong]
                app_unlocked_achievements_count,
                #[strong]
                cancelled_task,
                #[strong]
                update_autofill,
                #[weak]
                app_id,
                #[weak]
                app_achievement_count_value,
                #[weak]
                raw_model,
                #[weak]
                start_button,
                move |switch| {
                    let Some(achievement_object) =
                        list_item.item().and_downcast::<GAchievementObject>()
                    else {
                        return;
                    };
                    if !cancelled_task.load(std::sync::atomic::Ordering::Relaxed) {
                        dev_println!(
                            "CLIENT",
                            "Skipping switch toggle during timed unlock: {}",
                            achievement_object.name()
                        );
                        return;
                    }
                    if !switch.is_sensitive() {
                        return;
                    }
                    if switch.is_active() == achievement_object.is_achieved() {
                        return;
                    }

                    switch.set_sensitive(false);
                    let raw_model_len = raw_model.n_items();
                    let unlocked = switch.is_active();

                    achievement_object.set_is_achieved(unlocked);
                    let achievement_id = achievement_object.id();
                    let app_id_val = app_id.get().unwrap_or_default();
                    let handle = spawn_blocking(move || {
                        SetAchievement {
                            app_id: app_id_val,
                            achievement_id,
                            unlocked,
                            store: true,
                        }
                        .request()
                    });
                    MainContext::default().spawn_local(clone!(
                        #[strong]
                        app_unlocked_achievements_count,
                        #[strong]
                        update_autofill,
                        #[weak]
                        app_achievement_count_value,
                        #[weak]
                        switch,
                        #[weak]
                        achievement_object,
                        #[weak]
                        start_button,
                        async move {
                            let result = handle.await.expect("spawn_blocking task panicked");
                            match result {
                                Ok(_) => {
                                    let cur = app_unlocked_achievements_count.get();
                                    let new_unlocked = if unlocked { cur + 1 } else { cur - 1 };
                                    app_unlocked_achievements_count.set(new_unlocked);
                                    app_achievement_count_value.set_label(
                                        &format_achievement_progress(
                                            new_unlocked,
                                            raw_model_len as usize,
                                        ),
                                    );
                                    start_button
                                        .set_sensitive(new_unlocked != raw_model_len as usize);
                                    update_autofill();
                                }
                                Err(e) => {
                                    eprintln!("[CLIENT] Error setting achievement: {e}");
                                    achievement_object.set_is_achieved(!unlocked);
                                }
                            }
                            switch.set_sensitive(true);
                        }
                    ));
                }
            ));

            // Deferred toggle handler — pure queue manipulation.
            row.stage_toggle().connect_clicked(clone!(
                #[weak]
                list_item,
                #[strong]
                queue,
                #[weak]
                raw_model,
                #[weak]
                queue_label,
                #[weak]
                start_button,
                #[strong]
                app_unlocked_achievements_count,
                move |_| {
                    let Some(ach) = list_item.item().and_downcast::<GAchievementObject>() else {
                        return;
                    };
                    if ach.is_achieved() || ach.permission() != 0 {
                        return;
                    }
                    queue.toggle(&ach, &raw_model);
                    update_queue_label(&queue_label, &queue);
                    update_start_sensitive(
                        &start_button,
                        &queue,
                        app_unlocked_achievements_count.get(),
                        raw_model.n_items(),
                    );
                }
            ));
        }
    ));
}
