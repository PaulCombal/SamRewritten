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

use super::request::{Request, SetFloatStat, SetIntStat};
use super::stat::GStatObject;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::SignalHandlerId;
use gtk::glib::object::Cast;
use gtk::glib::translate::FromGlib;
use gtk::pango::EllipsizeMode;
use gtk::prelude::{
    BoxExt, GObjectPropertyExpressionExt, ListItemExt, ObjectExt, ToValue, WidgetExt,
};
use gtk::{
    Adjustment, Align, Box, ClosureExpression, FilterListModel, Frame, Label, ListItem, ListView,
    NoSelection, Orientation, ScrolledWindow, SignalListItemFactory, SpinButton, StringFilter,
    StringFilterMatchMode, Widget, glib,
};
use std::cell::RefCell;
use std::ffi::c_ulong;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn create_stats_view() -> (Frame, ListStore, StringFilter) {
    let stats_list_factory = SignalListItemFactory::new();
    let app_stats_model = ListStore::new::<GStatObject>();

    let app_stats_string_filter = StringFilter::builder()
        .expression(&GStatObject::this_expression("display-name"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let app_stats_filter_model = FilterListModel::builder()
        .model(&app_stats_model)
        .filter(&app_stats_string_filter)
        .build();
    let app_stats_selection_model = NoSelection::new(Option::<ListStore>::None);
    app_stats_selection_model.set_model(Some(&app_stats_filter_model));

    let app_stats_list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(&app_stats_selection_model)
        .factory(&stats_list_factory)
        .build();
    let app_stats_scrolled_window = ScrolledWindow::builder()
        .child(&app_stats_list_view)
        .vexpand(true)
        .build();

    stats_list_factory.connect_setup(move |_, list_item| {
        let adjustment = Adjustment::builder()
            .lower(i32::MIN as f64)
            .upper(i32::MAX as f64)
            .page_size(0.0)
            .build();

        let spin_button = SpinButton::builder().adjustment(&adjustment).build();

        let button_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::End)
            .build();
        button_box.append(&spin_button);
        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let name_label = Label::builder()
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();

        let stat_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        stat_box.append(&name_label);
        stat_box.append(&spacer);

        let icon_increment_only = gtk::Image::from_icon_name("go-up-symbolic");
        icon_increment_only.set_margin_end(8);
        icon_increment_only.set_tooltip_text(Some("Increment only"));
        stat_box.append(&icon_increment_only);

        let protected_icon = gtk::Image::from_icon_name("action-unavailable-symbolic");
        protected_icon.set_margin_end(8);
        protected_icon.set_tooltip_text(Some("This statistic is protected."));
        stat_box.append(&protected_icon);

        stat_box.append(&button_box);
        list_item.set_child(Some(&stat_box));

        // Expression bindings
        list_item
            .property_expression("item")
            .chain_property::<GStatObject>("display-name")
            .bind(&name_label, "label", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GStatObject>("current-value")
            .bind(&adjustment, "value", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GStatObject>("is-increment-only")
            .bind(&icon_increment_only, "visible", Widget::NONE);

        // Custom expressions
        let is_integer_expr = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("is-integer");

        let is_integer_expr_2 = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("is-integer");

        let is_increment_only_expr = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("is-increment-only");

        let original_value_expr = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("original-value");

        let permission_expr = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("permission");

        let permission_expr_2 = list_item
            .property_expression("item")
            .chain_property::<GStatObject>("permission");

        let adjustment_step_increment_closure =
            glib::RustClosure::new(|values: &[glib::Value]| {
                let is_integer = values
                    .get(1)
                    .and_then(|val| val.get::<bool>().ok())
                    .unwrap_or(false);
                let step_increment = if is_integer { 1.0 } else { 0.01 };
                Some(step_increment.to_value())
            });

        let adjustment_lower_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let original_value = values
                .get(1)
                .and_then(|val| val.get::<f64>().ok())
                .unwrap_or(0f64);
            let is_increment_only = values
                .get(2)
                .and_then(|val| val.get::<bool>().ok())
                .unwrap_or(false);

            let lower = if is_increment_only {
                original_value
            } else {
                i32::MIN as f64
            };
            Some(lower.to_value())
        });

        let spin_button_digits_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let is_integer = values
                .get(1)
                .and_then(|val| val.get::<bool>().ok())
                .unwrap_or(false);
            let digits: u32 = if is_integer { 0 } else { 2 };
            Some(digits.to_value())
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

        let adjustment_step_increment_expression =
            ClosureExpression::new::<f64>(&[is_integer_expr], adjustment_step_increment_closure);
        adjustment_step_increment_expression.bind(&adjustment, "step-increment", Widget::NONE);

        let adjustment_lower_expression = ClosureExpression::new::<f64>(
            &[original_value_expr, is_increment_only_expr],
            adjustment_lower_closure,
        );
        adjustment_lower_expression.bind(&adjustment, "lower", Widget::NONE);

        let spin_button_digits_expression =
            ClosureExpression::new::<u32>(&[is_integer_expr_2], spin_button_digits_closure);
        spin_button_digits_expression.bind(&spin_button, "digits", Widget::NONE);

        let permission_sensitive_expr =
            ClosureExpression::new::<bool>(&[permission_expr], permission_sensitive_closure);
        permission_sensitive_expr.bind(&spin_button, "sensitive", Widget::NONE);

        let permission_protected_expr =
            ClosureExpression::new::<bool>(&[permission_expr_2], permission_protected_closure);
        permission_protected_expr.bind(&protected_icon, "visible", Widget::NONE);
    });

    stats_list_factory.connect_bind(move |_, list_item| unsafe {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        let stat_object = list_item
            .item()
            .and_then(|item| item.downcast::<GStatObject>().ok())
            .expect("Item should be a GStatObject");

        let spin_button = list_item
            .child()
            .and_then(|child| child.downcast::<Box>().ok())
            .and_then(|stat_box| stat_box.last_child()) // This gets button_box
            .and_then(|button_box| button_box.downcast::<Box>().ok())
            .and_then(|button_box| button_box.last_child()) // This gets spin_button
            .and_then(|spin_button_widget| spin_button_widget.downcast::<SpinButton>().ok())
            .expect("Could not find SpinButton widget");

        let sender = RefCell::new(channel::<f64>().0);

        let handler_id = spin_button.connect_value_changed(move |button| {
            if sender.borrow_mut().send(button.value()).is_ok() {
                return;
            }
            let (new_sender, receiver) = channel();
            *sender.borrow_mut() = new_sender;
            let mut value = button.value();
            let integer_stat = stat_object.is_integer();
            let stat_id = stat_object.id().clone();
            let stat_object_clone = stat_object.clone();
            let app_id = stat_object.app_id().clone();

            glib::spawn_future_local(async move {
                let join_handle = spawn_blocking(move || {
                    while let Ok(new) = receiver.recv_timeout(Duration::from_millis(500)) {
                        // value = new; is not used, there can be floating point math imprecisions
                        value = (new * 100.0).round() / 100.0;
                    }

                    let res = if integer_stat {
                        SetIntStat {
                            app_id,
                            stat_id,
                            value: value as i32,
                        }
                        .request()
                    } else {
                        SetFloatStat {
                            app_id,
                            stat_id,
                            value: value as f32,
                        }
                        .request()
                    };

                    match res {
                        Ok(success) if success => (true, value),
                        _ => (false, value),
                    }
                });

                let (success, debounced_value) =
                    join_handle.await.expect("spawn_blocking task panicked");

                if success {
                    stat_object_clone.set_original_value(debounced_value);
                } else {
                    stat_object_clone.set_current_value(stat_object_clone.original_value());
                }
            });
        });

        spin_button.set_data("handler", handler_id.as_raw());
    });

    stats_list_factory.connect_unbind(move |_, list_item| unsafe {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");

        let spin_button = list_item
            .child()
            .and_then(|child| child.downcast::<Box>().ok())
            .and_then(|stat_box| stat_box.last_child()) // This gets button_box
            .and_then(|button_box| button_box.downcast::<Box>().ok())
            .and_then(|button_box| button_box.last_child()) // This gets spin_button
            .and_then(|spin_button_widget| spin_button_widget.downcast::<SpinButton>().ok())
            .expect("Could not find SpinButton widget");

        // Disconnect previous handler if it exists
        if let Some(handler_id) = spin_button.data("handler") {
            let ulong: c_ulong = *handler_id.as_ptr();
            let signal_handler = SignalHandlerId::from_glib(ulong);
            spin_button.disconnect(signal_handler);
        } else {
            println!("[CLIENT] Stat spinbox unbind failed");
        }
    });

    let app_stats_frame = Frame::builder()
        .margin_end(15)
        .margin_start(15)
        .margin_top(15)
        .margin_bottom(15)
        .child(&app_stats_scrolled_window)
        .build();

    (app_stats_frame, app_stats_model, app_stats_string_filter)
}
