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

use crate::dev_println;
use crate::gui_frontend::widgets::custom_progress_bar::CustomProgressBar;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::request::{Request, SetAchievement};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::translate::FromGlib;
use gtk::glib::{MainContext, SignalHandlerId, clone};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    Align, Box, ClosureExpression, Frame, Label, ListItem,
    ListView, NoSelection, Orientation, Overlay, ScrolledWindow,
    SignalListItemFactory, Stack, StackTransitionType, Switch, Widget, glib,
};
use std::cell::Cell;
use std::ffi::c_ulong;
use std::rc::Rc;

#[inline]
pub fn create_achievements_manual_view(
    app_id: &Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: &Rc<Cell<usize>>,
    filtered_model: &NoSelection,
    raw_model: &ListStore,
    app_achievement_count_value: &Label,
) -> (Frame,) {
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
            let is_sensitive = permission == 0;
            Some(is_sensitive.to_value())
        });
        let permission_protected_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let permission = values
                .get(1)
                .and_then(|val| val.get::<i32>().ok())
                .unwrap_or(0);
            let is_protected = permission != 0;
            Some(is_protected.to_value())
        });

        let visible_child_expr =
            ClosureExpression::new::<String>(&[is_achieved_expr], achieved_visible_icon_closure);
        let permission_sensitive_expr = ClosureExpression::new::<bool>(
            std::slice::from_ref(&permission_expr),
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
        #[weak]
        app_id,
        #[weak]
        app_achievement_count_value,
        #[weak]
        raw_model,
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
                #[weak]
                app_achievement_count_value,
                #[weak]
                raw_model,
                move |switch| {
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

                                    app_unlocked_achievements_count.set(new_unlocked_count);

                                    app_achievement_count_value.set_label(&format!(
                                        "{new_unlocked_count} / {raw_model_len}"
                                    ));

                                    let _lower = std::cmp::min(
                                        new_unlocked_count + 1,
                                        raw_model_len as usize,
                                    );
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


    let app_achievements_frame = Frame::builder()
        .margin_end(15)
        .margin_start(15)
        .margin_top(15)
        .margin_bottom(15)
        .child(&app_achievements_scrolled_window)
        .build();

    (
        app_achievements_frame,
    )
}
