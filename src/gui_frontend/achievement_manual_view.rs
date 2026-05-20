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
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::custom_progress_bar_widget::CustomProgressBar;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::mode_state::{GUnlockModeState, MODE_AUTOCOMMIT, MODE_DEFERRED};
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::request::{Request, SetAchievement};
use crate::gui_frontend::unlock_queue::{UnlockQueue, resolve_target_count};
use crate::gui_frontend::unlock_scheduler::{
    SPACING_EVEN, SPACING_RANDOM, compute_unlock_times_ms, run_timed_unlock, unlock_all_immediately,
};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use crate::utils::format::format_achievement_progress;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    Adjustment, Align, Box, Button, ClosureExpression, ConstantExpression, Frame, Grid,
    Label, ListBox, ListBoxRow, ListItem, ListView, NoSelection, Orientation, Overlay, Popover,
    ScrolledWindow, SelectionMode, SignalListItemFactory, SpinButton, Stack, StackTransitionType,
    Switch, ToggleButton, Widget, glib,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const STAGE_LABEL: &str = "Stage unlocks";

struct Header {
    container: ListBox,
    start_button: Button,
    queue_label: Label,
    auto_fill_button: Button,
}

struct Config {
    popover: Popover,
    count_spin: SpinButton,
    percent_spin: SpinButton,
    duration_spin: SpinButton,
    unit_percent: ToggleButton,
    spacing_random: ToggleButton,
}

#[inline]
fn create_header(mode_state: &Rc<GUnlockModeState>, config_popover: &Popover) -> Header {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .build();
    let hbox = Box::new(Orientation::Horizontal, 10);

    let mode_label = Label::new(Some(STAGE_LABEL));
    let mode_switch = Switch::builder().valign(Align::Center).build();

    let spacer = Box::builder()
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .build();

    let queue_label = Label::builder().build();
    let auto_fill_button = Button::builder().label("Auto-fill").build();
    let auto_fill_dropdown = Button::builder()
        .icon_name("pan-down-symbolic")
        .tooltip_text("Configuration")
        .build();
    // Anchor the popover to the dropdown button and drive it manually so both
    // halves of the linked group are plain `Button` widgets (same CSS name,
    // identical styling under .linked across themes).
    config_popover.set_parent(&auto_fill_dropdown);
    auto_fill_dropdown.connect_clicked(clone!(
        #[weak]
        config_popover,
        move |_| config_popover.popup()
    ));
    // Manually-parented popovers must be explicitly unparented before the
    // anchor widget is finalized, otherwise GTK warns about lingering children.
    auto_fill_dropdown.connect_destroy(clone!(
        #[strong]
        config_popover,
        move |_| config_popover.unparent()
    ));
    let auto_fill_group = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"])
        .build();
    auto_fill_group.append(&auto_fill_button);
    auto_fill_group.append(&auto_fill_dropdown);

    let start_button = Button::builder()
        .label("Start")
        .css_classes(["suggested-action"])
        .build();

    hbox.append(&mode_label);
    hbox.append(&mode_switch);
    hbox.append(&spacer);
    hbox.append(&queue_label);
    hbox.append(&auto_fill_group);
    hbox.append(&start_button);

    // mode_switch.active <-> mode_state.mode
    mode_switch.set_active(mode_state.mode() == MODE_DEFERRED);
    mode_switch.connect_active_notify(clone!(
        #[strong]
        mode_state,
        move |sw| {
            mode_state.set_mode(if sw.is_active() {
                MODE_DEFERRED
            } else {
                MODE_AUTOCOMMIT
            });
        }
    ));
    mode_state.connect_mode_notify(clone!(
        #[weak]
        mode_switch,
        move |state| {
            let want = state.mode() == MODE_DEFERRED;
            if mode_switch.is_active() != want {
                mode_switch.set_active(want);
            }
        }
    ));

    // Start, Auto-fill and the queue label are deferred-only.
    let visibility_apply = clone!(
        #[weak]
        start_button,
        #[weak]
        queue_label,
        #[weak]
        auto_fill_group,
        move |state: &GUnlockModeState| {
            let visible = state.mode() == MODE_DEFERRED;
            start_button.set_visible(visible);
            queue_label.set_visible(visible);
            auto_fill_group.set_visible(visible);
        }
    );
    visibility_apply(mode_state);
    mode_state.connect_mode_notify(move |state| visibility_apply(state));

    let row = ListBoxRow::builder()
        .child(&hbox)
        .activatable(false)
        .margin_end(5)
        .margin_start(5)
        .margin_top(5)
        .margin_bottom(5)
        .build();
    list.append(&row);

    Header {
        container: list,
        start_button,
        queue_label,
        auto_fill_button,
    }
}

#[inline]
fn create_config_popover(settings: &gtk::gio::Settings) -> Config {
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

    let unit_count_toggle = ToggleButton::builder().label("Count").build();
    let unit_percent_toggle = ToggleButton::builder()
        .label("Percent")
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
        .label("minutes (0 = instant)")
        .halign(Align::Start)
        .css_classes(["dim-label"])
        .build();
    let duration_row = Box::new(Orientation::Horizontal, 6);
    duration_row.append(&duration_spin);
    duration_row.append(&duration_minutes_label);

    let spacing_even_toggle = ToggleButton::builder().label("Even").hexpand(true).build();
    let spacing_random_toggle = ToggleButton::builder()
        .label("Random")
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
    let target_label = Label::builder().label("Target").halign(Align::End).build();
    let duration_label = Label::builder()
        .label("Spread over")
        .halign(Align::End)
        .build();
    let spacing_label = Label::builder().label("Spacing").halign(Align::End).build();
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
            let val = if btn.is_active() {
                "percent"
            } else {
                "count"
            };
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

fn update_queue_label(label: &Label, queue: &UnlockQueue) {
    let n = queue.len();
    label.set_label(&match n {
        0 => "No achievements staged".to_string(),
        1 => "1 staged".to_string(),
        n => format!("{n} staged"),
    });
}

fn update_start_sensitive(start_button: &Button, queue: &UnlockQueue, unlocked: usize, total: u32) {
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
        auto_fill_button.set_tooltip_text(Some(&format!(
            "Already at or above target ({})",
            format_achievement_progress(unlocked, total),
        )));
    } else {
        auto_fill_button.set_sensitive(true);
        auto_fill_button.set_tooltip_text(Some(&format!(
            "Auto-fill {to_add} achievement{} ({})",
            if to_add == 1 { "" } else { "s" },
            format_achievement_progress(unlocked, total),
        )));
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

    let queue = UnlockQueue::new();
    let cancelled_task = Arc::new(AtomicBool::new(true));

    let config = create_config_popover(&settings);
    let header = create_header(&mode_state, &config.popover);

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

    // Clearing the queue when leaving deferred mode keeps state honest.
    mode_state.connect_mode_notify(clone!(
        #[strong]
        queue,
        #[weak(rename_to = raw_model)]
        raw_model,
        #[weak(rename_to = queue_label)]
        header.queue_label,
        #[weak(rename_to = start_button)]
        header.start_button,
        move |state| {
            if state.mode() != MODE_DEFERRED {
                queue.clear(&raw_model);
                update_queue_label(&queue_label, &queue);
                start_button.set_sensitive(false);
            }
        }
    ));

    // Auto-fill main button: replace the queue based on the chosen target.
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

    // Start button: snapshot the queue, compute times, kick off the scheduler.
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

fn resolve_queue_to_objects(raw_model: &ListStore, ids: &[String]) -> Vec<GAchievementObject> {
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

#[allow(clippy::too_many_arguments)]
fn install_row_factory(
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

            // Autocommit trailing widget: protected icon + switch.
            let ac_protected_icon = gtk::Image::from_icon_name("action-unavailable-symbolic");
            ac_protected_icon.set_margin_end(8);
            ac_protected_icon.set_tooltip_text(Some("This achievement is protected."));
            let switch = Switch::builder().valign(Align::Center).build();
            let ac_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .valign(Align::Center)
                .build();
            ac_box.append(&ac_protected_icon);
            ac_box.append(&switch);

            // Deferred-pending trailing widget: protected icon + position label + icon-only
            // toggle. Putting the position in a sibling Label (instead of the button's own
            // label) keeps the button at a single, theme-stable size across all rows.
            let df_protected_icon = gtk::Image::from_icon_name("action-unavailable-symbolic");
            df_protected_icon.set_margin_end(8);
            df_protected_icon.set_tooltip_text(Some("This achievement is protected."));
            let position_label = Label::builder()
                .valign(Align::Center)
                .width_request(24)
                .xalign(1.0)
                .margin_end(6)
                .css_classes(["dim-label"])
                .build();
            let toggle = ToggleButton::builder()
                .valign(Align::Center)
                .css_classes(["circular"])
                .icon_name("list-add-symbolic")
                .tooltip_text("Click to stage this achievement; click again to remove.")
                .build();
            let df_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .valign(Align::Center)
                .halign(Align::End)
                .build();
            df_box.append(&df_protected_icon);
            df_box.append(&position_label);
            df_box.append(&toggle);

            // Deferred-done trailing widget: same icon-only toggle shape so rows line up.
            let done_toggle = ToggleButton::builder()
                .valign(Align::Center)
                .halign(Align::End)
                .css_classes(["circular"])
                .sensitive(false)
                .icon_name("emblem-ok-symbolic")
                .tooltip_text("Already unlocked.")
                .build();

            let trailing_stack = Stack::builder().build();
            trailing_stack.add_named(&ac_box, Some("autocommit"));
            trailing_stack.add_named(&df_box, Some("deferred-pending"));
            trailing_stack.add_named(&done_toggle, Some("deferred-done"));

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
            achievement_box.append(&trailing_stack);

            let overlay = Overlay::builder()
                .child(&global_percentage_progress_bar)
                .build();
            overlay.add_overlay(&achievement_box);
            overlay.set_measure_overlay(&achievement_box, true);
            list_item.set_child(Some(&overlay));

            // Standard property bindings.
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

            let is_achieved_expr = list_item
                .property_expression("item")
                .chain_property::<GAchievementObject>("is-achieved");
            let permission_expr = list_item
                .property_expression("item")
                .chain_property::<GAchievementObject>("permission");
            let queue_position_expr = list_item
                .property_expression("item")
                .chain_property::<GAchievementObject>("queue-position");
            let mode_expr =
                ConstantExpression::new(&*mode_state).chain_property::<GUnlockModeState>("mode");

            // Icon stack: locked vs unlocked.
            let icon_visible_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let is_achieved = values
                    .get(1)
                    .and_then(|v| v.get::<bool>().ok())
                    .unwrap_or(false);
                Some(if is_achieved { "normal" } else { "locked" }.to_value())
            });
            let icon_visible_expr =
                ClosureExpression::new::<String>(&[is_achieved_expr.clone()], icon_visible_closure);
            icon_visible_expr.bind(&icon_stack, "visible-child-name", Widget::NONE);

            // Trailing stack: autocommit / deferred-pending / deferred-done.
            let trailing_visible_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let mode = values
                    .get(1)
                    .and_then(|v| v.get::<String>().ok())
                    .unwrap_or_default();
                let is_achieved = values
                    .get(2)
                    .and_then(|v| v.get::<bool>().ok())
                    .unwrap_or(false);
                let name = if mode == MODE_AUTOCOMMIT {
                    "autocommit"
                } else if is_achieved {
                    "deferred-done"
                } else {
                    "deferred-pending"
                };
                Some(name.to_value())
            });
            let trailing_visible_expr = ClosureExpression::new::<String>(
                &[mode_expr.clone(), is_achieved_expr.clone()],
                trailing_visible_closure,
            );
            trailing_visible_expr.bind(&trailing_stack, "visible-child-name", Widget::NONE);

            // Protected/sensitive bindings, same logic as before but mirrored to both controls.
            let sensitive_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let permission = values.get(1).and_then(|v| v.get::<i32>().ok()).unwrap_or(0);
                Some((permission == 0).to_value())
            });
            let protected_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let permission = values.get(1).and_then(|v| v.get::<i32>().ok()).unwrap_or(0);
                Some((permission != 0).to_value())
            });
            let sensitive_expr = ClosureExpression::new::<bool>(
                std::slice::from_ref(&permission_expr),
                sensitive_closure,
            );
            let protected_expr =
                ClosureExpression::new::<bool>(&[permission_expr.clone()], protected_closure);
            sensitive_expr.bind(&switch, "sensitive", Widget::NONE);
            sensitive_expr.bind(&toggle, "sensitive", Widget::NONE);
            protected_expr.bind(&ac_protected_icon, "visible", Widget::NONE);
            protected_expr.bind(&df_protected_icon, "visible", Widget::NONE);

            // Toggle active state derived from queue position.
            let toggle_active_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let pos = values.get(1).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
                Some((pos != 0).to_value())
            });
            let toggle_active_expr = ClosureExpression::new::<bool>(
                &[queue_position_expr.clone()],
                toggle_active_closure,
            );
            toggle_active_expr.bind(&toggle, "active", Widget::NONE);

            // Sibling label shows the queue position (kept out of the button so the
            // button's intrinsic size stays constant across rows and themes).
            let position_text_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let pos = values.get(1).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
                let s = if pos == 0 {
                    String::new()
                } else {
                    pos.to_string()
                };
                Some(s.to_value())
            });
            let position_text_expr =
                ClosureExpression::new::<String>(&[queue_position_expr], position_text_closure);
            position_text_expr.bind(&position_label, "label", Widget::NONE);

            // Autocommit switch handler — same semantics as the old manual view.
            switch.connect_state_notify(clone!(
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
            toggle.connect_clicked(clone!(
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
