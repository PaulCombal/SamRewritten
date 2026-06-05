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
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Box, Button, Grid, Label, MenuButton, Orientation, Popover, SpinButton,
};

pub(super) struct CopyControls {
    pub(super) avatar_button: Button,
    pub(super) advanced_button: MenuButton,
    pub(super) start_button: Button,
    pub(super) max_gap_spin: SpinButton,
    pub(super) first_delay_spin: SpinButton,
    pub(super) preview_label: Label,
}

#[inline]
pub(super) fn create_copy_controls(settings: &gtk::gio::Settings) -> CopyControls {
    // Frameless so the rounded ShimmerImage avatar isn't boxed by a square frame.
    let avatar_default = gtk::Image::from_icon_name("avatar-default-symbolic");
    avatar_default.set_pixel_size(22);
    let avatar_button = Button::builder()
        .tooltip_text(tr("Choose a friend to copy timing from").as_str())
        .valign(Align::Center)
        .build();
    avatar_button.set_child(Some(&avatar_default));

    // Setting a custom child (rather than `.label()`) suppresses the dropdown
    // caret, which some themes (e.g. Greybird) render as a missing glyph.
    let advanced_button = MenuButton::builder().valign(Align::Center).build();
    advanced_button.set_child(Some(&Label::new(Some(tr("Advanced").as_str()))));
    let start_button = Button::builder()
        .label(tr("Start").as_str())
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();

    let max_gap_adj = Adjustment::builder()
        .lower(0.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .value(settings.int("copy-timing-max-gap-minutes").max(0) as f64)
        .build();
    let max_gap_spin = SpinButton::builder()
        .adjustment(&max_gap_adj)
        .digits(0)
        .hexpand(true)
        .build();
    let max_gap_row = Box::new(Orientation::Horizontal, 6);
    max_gap_row.append(&max_gap_spin);
    max_gap_row.append(
        &Label::builder()
            .label(tr("minutes").as_str())
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );

    let first_delay_adj = Adjustment::builder()
        .lower(0.0)
        .upper(i32::MAX as f64)
        .step_increment(1.0)
        .value(settings.int("copy-timing-first-delay-minutes").max(0) as f64)
        .build();
    let first_delay_spin = SpinButton::builder()
        .adjustment(&first_delay_adj)
        .digits(0)
        .hexpand(true)
        .build();
    let first_delay_row = Box::new(Orientation::Horizontal, 6);
    first_delay_row.append(&first_delay_spin);
    first_delay_row.append(
        &Label::builder()
            .label(tr("minutes").as_str())
            .halign(Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );

    let preview_label = Label::builder()
        .halign(Align::Start)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();

    let grid = Grid::builder().row_spacing(10).column_spacing(12).build();
    let end_label = |text: &str| Label::builder().label(text).halign(Align::End).build();
    grid.attach(&end_label(tr("Max gap").as_str()), 0, 0, 1, 1);
    grid.attach(&max_gap_row, 1, 0, 1, 1);
    grid.attach(&end_label(tr("First delay").as_str()), 0, 1, 1, 1);
    grid.attach(&first_delay_row, 1, 1, 1, 1);

    let body = Box::new(Orientation::Vertical, 10);
    body.set_margin_start(12);
    body.set_margin_end(12);
    body.set_margin_top(12);
    body.set_margin_bottom(12);
    body.set_width_request(300);
    body.append(&grid);
    body.append(&preview_label);

    let popover = Popover::builder().child(&body).build();
    advanced_button.set_popover(Some(&popover));

    max_gap_spin.connect_value_notify(clone!(
        #[strong]
        settings,
        move |sb| {
            let _ = settings.set_int("copy-timing-max-gap-minutes", sb.value_as_int());
        }
    ));
    first_delay_spin.connect_value_notify(clone!(
        #[strong]
        settings,
        move |sb| {
            let _ = settings.set_int("copy-timing-first-delay-minutes", sb.value_as_int());
        }
    ));

    CopyControls {
        avatar_button,
        advanced_button,
        start_button,
        max_gap_spin,
        first_delay_spin,
        preview_label,
    }
}
