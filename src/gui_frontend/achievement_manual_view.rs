// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
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

use crate::dev_println;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::achievement_view::count_unlocked_achievements;
use crate::gui_frontend::custom_progress_bar_widget::CustomProgressBar;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::request::{Request, SetAchievement, StoreStatsAndAchievements};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use crate::utils::format::format_seconds_to_hh_mm_ss;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::translate::FromGlib;
use gtk::glib::{MainContext, SignalHandlerId, clone};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Box, Button, ClosureExpression, Frame, Label, ListBox, ListBoxRow, ListItem,
    ListView, NoSelection, Orientation, Overlay, ScrolledWindow, SelectionMode,
    SignalListItemFactory, SpinButton, Stack, StackTransitionType, Switch, Widget, glib,
};
use std::cell::Cell;
use std::cmp::Ordering;
use std::ffi::c_ulong;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[inline]
fn create_header(
    app_id: &Rc<Cell<Option<u32>>>,
    achievement_views_stack: &Stack,
    raw_model: &ListStore,
    timed_raw_model: &ListStore,
    application: &MainApplication,
) -> (ListBox, Adjustment, SpinButton, Button, Arc<AtomicBool>) {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .build();
    let hbox = Box::new(Orientation::Horizontal, 10);

    let label_unlock = Label::new(Some("Get to"));
    let label_achievements_over = Label::new(Some("unlocked over"));
    let label_achievements_minutes = Label::new(Some("minutes"));

    let adjustment_achievements_count = Adjustment::builder()
        .lower(0.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .build();
    let spin_button_achievements_count = SpinButton::builder()
        .adjustment(&adjustment_achievements_count)
        .digits(0)
        .build();
    let adjustment_minutes_count = Adjustment::builder()
        .lower(0.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .build();
    let spin_button_minutes_count = SpinButton::builder()
        .adjustment(&adjustment_minutes_count)
        .digits(0)
        .build();

    let spacer = Box::builder()
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .build();
    let button_start = Button::builder().label("Start").build();
    let cancelled_task = Arc::new(AtomicBool::new(true));

    hbox.append(&label_unlock);
    hbox.append(&spin_button_achievements_count);
    hbox.append(&label_achievements_over);
    hbox.append(&spin_button_minutes_count);
    hbox.append(&label_achievements_minutes);
    hbox.append(&spacer);
    hbox.append(&button_start);

    button_start.connect_clicked(clone!(
        #[weak]
        raw_model,
        #[strong]
        timed_raw_model,
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        spin_button_minutes_count,
        #[weak]
        spin_button_achievements_count,
        #[weak]
        achievement_views_stack,
        #[weak]
        cancelled_task,
        move |_| {
            let unlocked_achievements = count_unlocked_achievements(&raw_model) as i32;
            let total_achievements = raw_model.n_items();
            let desired_achievements = spin_button_achievements_count.value_as_int();
            let desired_minutes = spin_button_minutes_count.value_as_int();
            let achievements_to_unlock_count = (desired_achievements - unlocked_achievements) as usize;
            let mut achievements_to_unlock = vec![];

            for achievement in &raw_model {
                if let Ok(obj) = achievement {
                    let g_achievement = obj.downcast::<GAchievementObject>().expect("Not a GAchievementObject");
                    if !g_achievement.is_achieved() && g_achievement.permission() & 2 == 0 {
                        achievements_to_unlock.push(g_achievement);
                    }
                }
            }

            achievements_to_unlock.sort_by(|a, b| {
                let percent_a = a.global_achieved_percent();
                let percent_b = b.global_achieved_percent();

                percent_b.partial_cmp(&percent_a).unwrap_or_else(|| {
                    if percent_a.is_nan() && percent_b.is_nan() {
                        Ordering::Equal
                    } else if percent_a.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
            });

            achievements_to_unlock.truncate(achievements_to_unlock_count);
            let app_id_int = app_id.get().expect("No App ID?");

            dev_println!("[CLIENT] Evaluation of automatic unlocking: unlocked: {unlocked_achievements}, total: {total_achievements}, desired: {desired_achievements}");
            if desired_minutes == 0 {
                dev_println!("[CLIENT] Unlock desired achievements immediately");
                for achievement_to_unlock in achievements_to_unlock {
                    let res = SetAchievement {
                        app_id: app_id_int,
                        achievement_id: achievement_to_unlock.id(),
                        unlocked: true,
                        store: false
                    }.request();

                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("[CLIENT] Failed to set achievement: {:?}", e);
                        }
                    }
                }

                let res = StoreStatsAndAchievements {app_id: app_id_int}.request();
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("[CLIENT] Failed to store stats and achievements: {:?}", e);
                    }
                }

                application.activate_action("refresh_achievements_list", None);
                return;
            }

            timed_raw_model.remove_all();
            timed_raw_model.extend_from_slice(&achievements_to_unlock);
            cancelled_task.store(false, std::sync::atomic::Ordering::Relaxed);

            MainContext::default().spawn_local(clone!(
                #[strong]
                timed_raw_model,
                async move {
                    let mut elapsed = 0usize;
                    let need_elapsed = (desired_minutes * 60 * 1000) as usize;
                    let ms_per_achievement = need_elapsed / achievements_to_unlock_count;
                    let refresh_rate = std::cmp::min(1000, ms_per_achievement);
                    let refreshes_per_achievement = ms_per_achievement / refresh_rate;
                    let refresh_rate = std::time::Duration::from_millis(refresh_rate as u64);
                    let mut next_ach_to_unlock_index = 0usize;
                    let mut refreshes_without_unlock = 0usize;

                    while next_ach_to_unlock_index < achievements_to_unlock_count {
                        if cancelled_task.load(std::sync::atomic::Ordering::Relaxed) {
                            dev_println!("[CLIENT] Timed unlock task cancelled");
                            timed_raw_model.remove_all();
                            break;
                        }

                        glib::timeout_future(refresh_rate).await;

                        refreshes_without_unlock += 1;
                        if refreshes_without_unlock >= refreshes_per_achievement {
                            let achievement = &achievements_to_unlock[next_ach_to_unlock_index];
                            dev_println!("[CLIENT] Timed unlock of {}", achievement.name());
                            achievement.set_is_achieved(true);

                            let achievement_id = achievement.id();
                            let result = spawn_blocking(move || {
                                SetAchievement {
                                    app_id: app_id_int,
                                    achievement_id,
                                    unlocked: true,
                                    store: true
                                }
                                .request()
                            }).await;


                            match result {
                                Ok(response) => {
                                    dev_println!("[CLIENT] Achievement unlocking result: {:?}", response);
                                }
                                Err(e) => eprintln!("[CLIENT] Achievement unlocking failed: {:?}", e),
                            }

                            next_ach_to_unlock_index += 1;
                            refreshes_without_unlock = 0;
                        }

                        elapsed += refresh_rate.as_millis() as usize;

                        for i in 0..achievements_to_unlock.len() {
                            let target_elapsed_ms_for_ach = (i + 1) * ms_per_achievement;
                            let remaining_ms = target_elapsed_ms_for_ach.saturating_sub(elapsed);
                            let remaining_seconds = remaining_ms / 1000;

                            if remaining_seconds == 0 {
                                achievements_to_unlock[i].set_time_until_unlock("OK");
                            }
                            else {
                                let timer_str = format_seconds_to_hh_mm_ss(remaining_seconds);
                                achievements_to_unlock[i].set_time_until_unlock(timer_str);
                            }
                        }
                    }

                    dev_println!("[CLIENT] Timed unlock task finished");
                }
            ));

            achievement_views_stack.set_visible_child_name("automatic");
        }
    ));

    let list_box_row = ListBoxRow::builder()
        .child(&hbox)
        .activatable(false)
        .margin_end(5)
        .margin_start(5)
        .margin_top(5)
        .margin_bottom(5)
        .build();
    list.append(&list_box_row);

    (
        list,
        adjustment_achievements_count,
        spin_button_achievements_count,
        button_start,
        cancelled_task,
    )
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
) -> (Frame, Adjustment, SpinButton, Button, Arc<AtomicBool>) {
    let (
        header,
        header_achievements_adjustment,
        header_achievements_spinbox,
        header_achievements_start,
        cancel_timed_unlock,
    ) = create_header(
        &app_id,
        &achievement_views_stack,
        raw_model,
        timed_raw_model,
        application,
    );

    let achievements_list_factory = SignalListItemFactory::new();

    let app_achievements_list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(filtered_model)
        .factory(&achievements_list_factory)
        .build();
    let app_achievements_scrolled_window = ScrolledWindow::builder()
        .child(&app_achievements_list_view)
        .vexpand(true)
        .build();

    achievements_list_factory.connect_setup(move |_, list_item| {
        let normal_icon = ShimmerImage::new();
        normal_icon.set_size_request(32, 32);
        let locked_icon = ShimmerImage::new();
        locked_icon.set_size_request(32, 32);

        let icon_stack = Stack::builder()
            .transition_type(StackTransitionType::RotateLeftRight)
            .build();
        icon_stack.add_named(&normal_icon, Some("normal"));
        icon_stack.add_named(&locked_icon, Some("locked"));

        let icon_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Start)
            .margin_end(8)
            .build();
        icon_box.append(&icon_stack);

        let protected_icon = gtk::Image::from_icon_name("action-unavailable-symbolic");
        protected_icon.set_margin_end(8);
        protected_icon.set_tooltip_text(Some("This achievement is protected."));

        let switch = Switch::builder().valign(Align::Center).build();

        let switch_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .valign(Align::Start)
            .build();
        switch_box.append(&protected_icon);
        switch_box.append(&switch);

        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let name_label = Label::builder()
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let description_label = Label::builder()
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let label_box = Box::builder().orientation(Orientation::Vertical).build();
        let global_percentage_progress_bar = CustomProgressBar::new();
        label_box.append(&name_label);
        label_box.append(&description_label);
        let achievement_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        achievement_box.append(&icon_box);
        achievement_box.append(&label_box);
        achievement_box.append(&spacer);
        achievement_box.append(&switch_box);

        let overlay = Overlay::builder()
            .child(&global_percentage_progress_bar)
            .build();
        overlay.add_overlay(&achievement_box);
        overlay.set_measure_overlay(&achievement_box, true);
        list_item.set_child(Some(&overlay));

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("name")
            .bind(&name_label, "label", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("description")
            .bind(&description_label, "label", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-normal")
            .bind(&normal_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-locked")
            .bind(&locked_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is-achieved")
            .bind(&switch, "active", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent")
            .bind(&global_percentage_progress_bar, "value", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent-ok")
            .bind(&global_percentage_progress_bar, "visible", Widget::NONE);

        // Custom expressions
        let is_achieved_expr = list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is-achieved");
        let permission_expr = list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("permission");

        let achieved_visible_icon_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let is_achieved = values
                .get(1)
                .and_then(|val| val.get::<bool>().ok())
                .unwrap_or(false);
            let child_name = if is_achieved { "normal" } else { "locked" };
            Some(child_name.to_value())
        });

        let permission_sensitive_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let permission = values
                .get(1)
                .and_then(|val| val.get::<i32>().ok())
                .unwrap_or(0);
            let is_sensitive = (permission & 2) == 0;
            Some(is_sensitive.to_value())
        });
        let permission_protected_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let permission = values
                .get(1)
                .and_then(|val| val.get::<i32>().ok())
                .unwrap_or(0);
            let is_protected = (permission & 2) != 0;
            Some(is_protected.to_value())
        });

        let visible_child_expr =
            ClosureExpression::new::<String>(&[is_achieved_expr], achieved_visible_icon_closure);
        let permission_sensitive_expr = ClosureExpression::new::<bool>(
            &[permission_expr.clone()],
            permission_sensitive_closure,
        );
        let permission_protected_expr =
            ClosureExpression::new::<bool>(&[permission_expr], permission_protected_closure);

        visible_child_expr.bind(&icon_stack, "visible-child-name", Widget::NONE);
        permission_sensitive_expr.bind(&switch, "sensitive", Widget::NONE);
        permission_protected_expr.bind(&protected_icon, "visible", Widget::NONE);
    });

    achievements_list_factory.connect_bind(clone!(
        #[strong]
        app_unlocked_achievements_count,
        #[strong]
        cancel_timed_unlock,
        #[weak]
        app_id,
        #[weak]
        app_achievement_count_value,
        #[weak]
        raw_model,
        #[weak]
        header_achievements_adjustment,
        #[weak]
        header_achievements_spinbox,
        #[weak]
        header_achievements_start,
        move |_, list_item| unsafe {
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            let achievement_object = list_item
                .item()
                .and_then(|item| item.downcast::<GAchievementObject>().ok())
                .expect("Item should be a GAchievementObject");

            let switch = list_item
                .child()
                .and_then(|child| child.downcast::<Overlay>().ok())
                .and_then(|overlay| overlay.last_child())
                .and_then(|main_box| main_box.last_child())
                .and_then(|switch_box| switch_box.downcast::<Box>().ok())
                .and_then(|switch_box| switch_box.last_child())
                .and_then(|switch| switch.downcast::<Switch>().ok())
                .expect("achievements_list_factory::connect_bind: Could not find Switch widget");

            let app_id = app_id.get().unwrap_or_default();
            let achievement_id = achievement_object.id().clone();

            let handler_id = switch.connect_state_notify(clone!(
                #[strong]
                app_unlocked_achievements_count,
                #[strong]
                cancel_timed_unlock,
                #[weak]
                app_achievement_count_value,
                #[weak]
                raw_model,
                #[weak]
                header_achievements_adjustment,
                #[weak]
                header_achievements_spinbox,
                #[weak]
                header_achievements_start,
                move |switch| {
                    if cancel_timed_unlock.load(std::sync::atomic::Ordering::Relaxed) == false {
                        dev_println!("[CLIENT] Not unlocking achievement after switch callback (automatic unlocking in progress): {}", achievement_object.name());
                        return;
                    }
                    if !switch.is_sensitive() {
                        dev_println!("[CLIENT] Switch flipped when not sensitive.. WARNING");
                        return;
                    }
                    switch.set_sensitive(false);
                    let raw_model_len = raw_model.n_items();
                    let unlocked = switch.is_active();

                    dev_println!("[CLIENT] Setting achievement after switch callback: {} ({})", achievement_object.name(), unlocked);

                    achievement_object.set_is_achieved(unlocked);
                    let achievement_id = achievement_id.clone();
                    let handle = spawn_blocking(move || {
                        SetAchievement {
                            app_id,
                            achievement_id,
                            unlocked,
                            store: true
                        }
                        .request()
                    });
                    MainContext::default().spawn_local(clone!(
                        #[strong]
                        app_unlocked_achievements_count,
                        #[weak]
                        app_achievement_count_value,
                        #[weak]
                        switch,
                        #[weak]
                        achievement_object,
                        #[weak]
                        header_achievements_start,
                        async move {
                            let result = handle.await.expect("spawn_blocking task panicked");

                            match result {
                                Ok(_) => {
                                    let unlocked_achievements_count_value =
                                        app_unlocked_achievements_count.get();

                                    let new_unlocked_count = if unlocked {
                                        unlocked_achievements_count_value + 1
                                    } else {
                                        unlocked_achievements_count_value - 1
                                    };

                                    header_achievements_start.set_sensitive(
                                        new_unlocked_count != raw_model_len as usize,
                                    );
                                    app_unlocked_achievements_count.set(new_unlocked_count);

                                    app_achievement_count_value.set_label(&format!(
                                        "{new_unlocked_count} / {raw_model_len}"
                                    ));

                                    let lower = std::cmp::min(
                                        new_unlocked_count + 1,
                                        raw_model_len as usize,
                                    );
                                    header_achievements_adjustment.set_lower(lower as f64);

                                    let spinbox_value =
                                        header_achievements_spinbox.value_as_int() as usize;
                                    let spinbox_value =
                                        std::cmp::max(spinbox_value, new_unlocked_count + 1);
                                    let spinbox_value =
                                        std::cmp::min(spinbox_value, raw_model_len as usize);
                                    header_achievements_spinbox.set_value(spinbox_value as f64);
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

            switch.set_data("handler", handler_id.as_raw());
        }
    ));

    achievements_list_factory.connect_unbind(move |_, list_item| unsafe {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");

        let switch = list_item
            .child()
            .and_then(|child| child.downcast::<Overlay>().ok())
            .and_then(|overlay| overlay.last_child())
            .and_then(|main_box| main_box.last_child())
            .and_then(|switch_box| switch_box.downcast::<Box>().ok())
            .and_then(|switch_box| switch_box.last_child())
            .and_then(|switch| switch.downcast::<Switch>().ok())
            .expect("achievements_list_factory::connect_unbind: Could not find Switch widget");

        // Disconnect handler when item is unbound
        if let Some(handler_id) = switch.data("handler") {
            let ulong: c_ulong = *handler_id.as_ptr();
            let signal_handler = SignalHandlerId::from_glib(ulong);
            switch.disconnect(signal_handler);
        } else {
            eprintln!("[CLIENT] Achievement switch unbind failed");
        }
    });

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&header);
    vbox.append(&app_achievements_scrolled_window);
    let app_achievements_frame = Frame::builder()
        .margin_end(15)
        .margin_start(15)
        .margin_top(15)
        .margin_bottom(15)
        .child(&vbox)
        .build();

    (
        app_achievements_frame,
        header_achievements_adjustment,
        header_achievements_spinbox,
        header_achievements_start,
        cancel_timed_unlock,
    )
}
