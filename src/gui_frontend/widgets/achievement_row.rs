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
use crate::gui_frontend::gobjects::mode_state::{
    GUnlockModeState, MODE_AUTOCOMMIT, MODE_COPY_TIMING, MODE_DEFERRED,
};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{ClosureExpression, ConstantExpression, ListItem, Switch, ToggleButton, Widget};

glib::wrapper! {
    pub struct AchievementRow(ObjectSubclass<imp::AchievementRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for AchievementRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl AchievementRow {
    pub fn new() -> Self {
        Self::default()
    }

    /// The autocommit switch; the factory attaches the instant-unlock handler.
    pub fn switch(&self) -> Switch {
        self.imp().switch.clone()
    }

    /// The deferred-mode stage toggle; the factory attaches the queue handler.
    pub fn stage_toggle(&self) -> ToggleButton {
        self.imp().toggle.clone()
    }

    /// Bind every display-only property of this row to `list_item`'s achievement
    /// and the shared `mode_state`. All bindings react to the list item's `item`
    /// changing, so this is called once at row setup and tracks recycling.
    pub fn bind_display(&self, list_item: &ListItem, mode_state: &GUnlockModeState) {
        let imp = self.imp();

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("name")
            .bind(&imp.name_label, "label", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("description")
            .bind(&imp.description_label, "label", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-normal")
            .bind(&imp.normal_icon, "url", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-locked")
            .bind(&imp.locked_icon, "url", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is-achieved")
            .bind(&imp.switch, "active", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent")
            .bind(&imp.progress_bar, "value", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent-ok")
            .bind(&imp.progress_bar, "visible", Widget::NONE);

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
            ConstantExpression::new(mode_state).chain_property::<GUnlockModeState>("mode");

        let icon_visible_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let is_achieved = values
                .get(1)
                .and_then(|v| v.get::<bool>().ok())
                .unwrap_or(false);
            Some(if is_achieved { "normal" } else { "locked" }.to_value())
        });
        let icon_visible_expr =
            ClosureExpression::new::<String>(&[is_achieved_expr.clone()], icon_visible_closure);
        icon_visible_expr.bind(&imp.icon_stack, "visible-child-name", Widget::NONE);

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
        trailing_visible_expr.bind(&imp.trailing_stack, "visible-child-name", Widget::NONE);

        // Sensitivity + protected icon, mirrored to the switch and the toggle.
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
        sensitive_expr.bind(&imp.switch, "sensitive", Widget::NONE);
        sensitive_expr.bind(&imp.toggle, "sensitive", Widget::NONE);
        protected_expr.bind(&imp.ac_protected_icon, "visible", Widget::NONE);
        protected_expr.bind(&imp.df_protected_icon, "visible", Widget::NONE);

        let toggle_active_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let pos = values.get(1).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
            Some((pos != 0).to_value())
        });
        let toggle_active_expr =
            ClosureExpression::new::<bool>(&[queue_position_expr.clone()], toggle_active_closure);
        toggle_active_expr.bind(&imp.toggle, "active", Widget::NONE);

        // The stage toggle is interactive only in deferred mode; copy-timing
        // reuses the deferred row (to show the planned position) but hides it,
        // since the plan comes from the friend, not the user's clicks.
        let toggle_visible_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let mode = values
                .get(1)
                .and_then(|v| v.get::<String>().ok())
                .unwrap_or_default();
            Some((mode == MODE_DEFERRED).to_value())
        });
        let toggle_visible_expr =
            ClosureExpression::new::<bool>(&[mode_expr.clone()], toggle_visible_closure);
        toggle_visible_expr.bind(&imp.toggle, "visible", Widget::NONE);

        // Copy-timing mode shows "<position> • <hh:mm>"; deferred shows just the
        // position. (The label is a sibling of the button so the button's size
        // stays constant across rows and themes.)
        let position_text_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let mode = values
                .get(1)
                .and_then(|v| v.get::<String>().ok())
                .unwrap_or_default();
            let pos = values.get(2).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
            let time = values
                .get(3)
                .and_then(|v| v.get::<String>().ok())
                .unwrap_or_default();
            let s = if pos == 0 {
                String::new()
            } else if mode == MODE_COPY_TIMING {
                format!("{pos} • {time}")
            } else {
                pos.to_string()
            };
            Some(s.to_value())
        });
        let time_until_unlock_expr = list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("time-until-unlock");
        let position_text_expr = ClosureExpression::new::<String>(
            &[
                mode_expr.clone(),
                queue_position_expr,
                time_until_unlock_expr,
            ],
            position_text_closure,
        );
        position_text_expr.bind(&imp.position_label, "label", Widget::NONE);
    }
}

mod imp {
    use crate::gui_frontend::custom_progress_bar_widget::CustomProgressBar;
    use crate::gui_frontend::i18n::tr;
    use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
    use gtk::glib;
    use gtk::pango::EllipsizeMode;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{
        Align, Box, Image, Label, Orientation, Overlay, Stack, StackTransitionType, Switch,
        ToggleButton,
    };

    pub struct AchievementRow {
        pub overlay: Overlay,
        pub normal_icon: ShimmerImage,
        pub locked_icon: ShimmerImage,
        pub icon_stack: Stack,
        pub name_label: Label,
        pub description_label: Label,
        pub progress_bar: CustomProgressBar,
        pub trailing_stack: Stack,
        pub ac_protected_icon: Image,
        pub switch: Switch,
        pub df_protected_icon: Image,
        pub position_label: Label,
        pub toggle: ToggleButton,
        pub done_toggle: ToggleButton,
    }

    impl Default for AchievementRow {
        fn default() -> Self {
            let normal_icon = ShimmerImage::new();
            normal_icon.set_size_request(32, 32);
            let locked_icon = ShimmerImage::new();
            locked_icon.set_size_request(32, 32);
            let icon_stack = Stack::builder()
                .transition_type(StackTransitionType::RotateLeftRight)
                .build();
            icon_stack.add_named(&normal_icon, Some("normal"));
            icon_stack.add_named(&locked_icon, Some("locked"));

            let ac_protected_icon = Image::from_icon_name("action-unavailable-symbolic");
            ac_protected_icon.set_margin_end(8);
            ac_protected_icon.set_tooltip_text(Some(tr("This achievement is protected.").as_str()));
            let switch = Switch::builder().valign(Align::Center).build();

            let df_protected_icon = Image::from_icon_name("action-unavailable-symbolic");
            df_protected_icon.set_margin_end(8);
            df_protected_icon.set_tooltip_text(Some(tr("This achievement is protected.").as_str()));
            // Putting the position in a sibling Label (instead of the button's own
            // label) keeps the button at a single, theme-stable size across rows.
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
                .tooltip_text(
                    tr("Click to stage this achievement; click again to remove.").as_str(),
                )
                .build();
            // Same icon-only toggle shape so rows line up.
            let done_toggle = ToggleButton::builder()
                .valign(Align::Center)
                .halign(Align::End)
                .css_classes(["circular"])
                .sensitive(false)
                .icon_name("object-select-symbolic")
                .tooltip_text(tr("Already unlocked.").as_str())
                .build();

            let name_label = Label::builder()
                .ellipsize(EllipsizeMode::End)
                .halign(Align::Start)
                .build();
            let description_label = Label::builder()
                .ellipsize(EllipsizeMode::End)
                .halign(Align::Start)
                .build();

            Self {
                overlay: Overlay::new(),
                normal_icon,
                locked_icon,
                icon_stack,
                name_label,
                description_label,
                progress_bar: CustomProgressBar::new(),
                trailing_stack: Stack::new(),
                ac_protected_icon,
                switch,
                df_protected_icon,
                position_label,
                toggle,
                done_toggle,
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AchievementRow {
        const NAME: &'static str = "SamAchievementRow";
        type Type = super::AchievementRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }
    }

    impl ObjectImpl for AchievementRow {
        fn constructed(&self) {
            self.parent_constructed();

            let icon_box = Box::builder()
                .orientation(Orientation::Vertical)
                .halign(Align::Start)
                .margin_end(8)
                .build();
            icon_box.append(&self.icon_stack);

            let ac_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .valign(Align::Center)
                .build();
            ac_box.append(&self.ac_protected_icon);
            ac_box.append(&self.switch);

            let df_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .valign(Align::Center)
                .halign(Align::End)
                .build();
            df_box.append(&self.df_protected_icon);
            df_box.append(&self.position_label);
            df_box.append(&self.toggle);

            self.trailing_stack.add_named(&ac_box, Some("autocommit"));
            self.trailing_stack
                .add_named(&df_box, Some("deferred-pending"));
            self.trailing_stack
                .add_named(&self.done_toggle, Some("deferred-done"));

            let spacer = Box::builder()
                .orientation(Orientation::Horizontal)
                .hexpand(true)
                .build();
            let label_box = Box::builder().orientation(Orientation::Vertical).build();
            label_box.append(&self.name_label);
            label_box.append(&self.description_label);

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
            achievement_box.append(&self.trailing_stack);

            self.overlay.set_child(Some(&self.progress_bar));
            self.overlay.add_overlay(&achievement_box);
            self.overlay.set_measure_overlay(&achievement_box, true);
            self.overlay.set_parent(&*self.obj());
        }

        fn dispose(&self) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for AchievementRow {}
}
