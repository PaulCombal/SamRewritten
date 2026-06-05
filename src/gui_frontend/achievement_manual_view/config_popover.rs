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

use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::unlock_scheduler::{SPACING_EVEN, SPACING_RANDOM};
use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Box, Grid, Label, Orientation, Popover, SpinButton, Stack, ToggleButton,
};

pub(super) struct Config {
    pub(super) popover: Popover,
    pub(super) count_spin: SpinButton,
    pub(super) percent_spin: SpinButton,
    pub(super) duration_spin: SpinButton,
    pub(super) unit_percent: ToggleButton,
    pub(super) spacing_random: ToggleButton,
}

#[inline]
pub(super) fn create_config_popover(settings: &gtk::gio::Settings) -> Config {
    let count_adjustment = Adjustment::builder()
        .lower(1.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .value(settings.int("auto-fill-count").max(1) as f64)
        .build();
    let count_spin = SpinButton::builder()
        .adjustment(&count_adjustment)
        .digits(0)
        .hexpand(true)
        .build();

    let percent_adjustment = Adjustment::builder()
        .lower(0.0)
        .upper(100.0)
        .step_increment(1.0)
        .value(settings.double("auto-fill-percent").clamp(0.0, 100.0))
        .build();
    let percent_spin = SpinButton::builder()
        .adjustment(&percent_adjustment)
        .digits(1)
        .hexpand(true)
        .build();

    let target_stack = Stack::builder().hexpand(true).build();
    target_stack.add_named(&count_spin, Some("count"));
    target_stack.add_named(&percent_spin, Some("percent"));

    let unit_count_toggle = ToggleButton::builder().label(tr("Count").as_str()).build();
    let unit_percent_toggle = ToggleButton::builder()
        .label(tr("Percent").as_str())
        .group(&unit_count_toggle)
        .build();
    let unit_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"].as_slice())
        .build();
    unit_box.append(&unit_count_toggle);
    unit_box.append(&unit_percent_toggle);
    let initial_unit = settings.string("auto-fill-unit").to_string();
    let is_percent = initial_unit == "percent";
    if is_percent {
        unit_percent_toggle.set_active(true);
    } else {
        unit_count_toggle.set_active(true);
    }
    target_stack.set_visible_child_name(if is_percent { "percent" } else { "count" });

    let target_row = Box::new(Orientation::Horizontal, 6);
    target_row.append(&target_stack);
    target_row.append(&unit_box);

    let duration_adjustment = Adjustment::builder()
        .lower(0.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .value(settings.int("unlock-duration-minutes").max(0) as f64)
        .build();
    let duration_spin = SpinButton::builder()
        .adjustment(&duration_adjustment)
        .digits(0)
        .hexpand(true)
        .build();
    let duration_minutes_label = Label::builder()
        .label(tr("minutes (0 = instant)").as_str())
        .halign(Align::Start)
        .css_classes(["dim-label"])
        .build();
    let duration_row = Box::new(Orientation::Horizontal, 6);
    duration_row.append(&duration_spin);
    duration_row.append(&duration_minutes_label);

    let spacing_even_toggle = ToggleButton::builder()
        .label(tr("Even").as_str())
        .hexpand(true)
        .build();
    let spacing_random_toggle = ToggleButton::builder()
        .label(tr("Random").as_str())
        .hexpand(true)
        .group(&spacing_even_toggle)
        .build();
    let spacing_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"].as_slice())
        .hexpand(true)
        .build();
    spacing_box.append(&spacing_even_toggle);
    spacing_box.append(&spacing_random_toggle);
    let initial_spacing = settings.string("unlock-spacing").to_string();
    if initial_spacing == SPACING_RANDOM {
        spacing_random_toggle.set_active(true);
    } else {
        spacing_even_toggle.set_active(true);
    }

    let grid = Grid::builder().row_spacing(10).column_spacing(12).build();
    let target_label = Label::builder()
        .label(tr("Target").as_str())
        .halign(Align::End)
        .build();
    let duration_label = Label::builder()
        .label(tr("Spread over").as_str())
        .halign(Align::End)
        .build();
    let spacing_label = Label::builder()
        .label(tr("Spacing").as_str())
        .halign(Align::End)
        .build();
    grid.attach(&target_label, 0, 0, 1, 1);
    grid.attach(&target_row, 1, 0, 1, 1);
    grid.attach(&duration_label, 0, 1, 1, 1);
    grid.attach(&duration_row, 1, 1, 1, 1);
    grid.attach(&spacing_label, 0, 2, 1, 1);
    grid.attach(&spacing_box, 1, 2, 1, 1);

    let body = Box::new(Orientation::Vertical, 10);
    body.set_margin_start(12);
    body.set_margin_end(12);
    body.set_margin_top(12);
    body.set_margin_bottom(12);
    body.set_width_request(320);
    body.append(&grid);

    let popover = Popover::builder().child(&body).build();

    count_spin.connect_value_notify(clone!(
        #[strong]
        settings,
        move |sb| {
            let _ = settings.set_int("auto-fill-count", sb.value_as_int());
        }
    ));
    percent_spin.connect_value_notify(clone!(
        #[strong]
        settings,
        move |sb| {
            let _ = settings.set_double("auto-fill-percent", sb.value());
        }
    ));
    duration_spin.connect_value_notify(clone!(
        #[strong]
        settings,
        move |sb| {
            let _ = settings.set_int("unlock-duration-minutes", sb.value_as_int());
        }
    ));
    unit_percent_toggle.connect_toggled(clone!(
        #[strong]
        settings,
        #[weak]
        target_stack,
        move |btn| {
            let val = if btn.is_active() { "percent" } else { "count" };
            let _ = settings.set_string("auto-fill-unit", val);
            target_stack.set_visible_child_name(val);
        }
    ));
    spacing_random_toggle.connect_toggled(clone!(
        #[strong]
        settings,
        move |btn| {
            let val = if btn.is_active() {
                SPACING_RANDOM
            } else {
                SPACING_EVEN
            };
            let _ = settings.set_string("unlock-spacing", val);
        }
    ));

    Config {
        popover,
        count_spin,
        percent_spin,
        duration_spin,
        unit_percent: unit_percent_toggle,
        spacing_random: spacing_random_toggle,
    }
}
