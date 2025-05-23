use std::cell::Cell;
use std::rc::Rc;
use gtk::gio::{spawn_blocking, ListStore};
use gtk::glib::{clone, MainContext};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{glib, Align, Box, FilterListModel, Label, ListBox, Orientation, SelectionMode, Stack, StackTransitionType, StringFilter, StringFilterMatchMode, Switch};
use crate::frontend::achievement::GAchievementObject;
use crate::frontend::request::{Request, SetAchievement};
use crate::frontend::shimmer_image::ShimmerImage;

pub fn create_achievements_view(app_id: Rc<Cell<Option<u32>>>) -> (ListBox, ListStore, StringFilter) {
    let app_achievements_model = ListStore::new::<GAchievementObject>();
    let app_achievement_string_filter = StringFilter::builder()
        .expression(&GAchievementObject::this_expression("search-text"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let app_achievement_filter_model = FilterListModel::builder()
        .model(&app_achievements_model)
        .filter(&app_achievement_string_filter)
        .build();
    let app_achievements_list = ListBox::builder()
        .show_separators(true)
        .build();
    app_achievements_list.set_selection_mode(SelectionMode::None);

    let app_id_clone = app_id.clone();
    app_achievements_list.bind_model(Some(&app_achievement_filter_model), move |item| {
        let achievement = item.downcast_ref::<GAchievementObject>()
            .expect("Needs to be a GSteamAppObject");

        let normal_icon = ShimmerImage::new();
        normal_icon.set_url(achievement.icon_normal().as_str());
        normal_icon.set_size_request(32, 32);
        let locked_icon = ShimmerImage::new();
        locked_icon.set_size_request(32, 32);
        locked_icon.set_url(achievement.icon_locked().as_str());
        let icon_stack = Stack::builder()
            .transition_type(StackTransitionType::RotateLeftRight)
            .build();
        icon_stack.add_named(&normal_icon, Some("normal"));
        icon_stack.add_named(&locked_icon, Some("locked"));
        icon_stack.set_visible_child_name(if achievement.is_achieved() { "normal" } else { "locked" });
        let icon_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Start)
            .margin_end(8)
            .build();
        icon_box.append(&icon_stack);

        let switch = Switch::builder()
            .active(achievement.is_achieved())
            .valign(Align::Center)
            .build();

        let app_id = app_id_clone.get().unwrap_or_default();
        let achievement_id = achievement.id().clone();
        switch.connect_state_notify(clone!(#[weak] icon_stack, move |switch| {
            if switch.is_active() {
                icon_stack.set_visible_child_name("normal");
            } else {
                icon_stack.set_visible_child_name("locked");
            }
            if !switch.is_sensitive() { return }
            switch.set_sensitive(false); 
            let unlocked = switch.is_active();
            let achievement_id = achievement_id.clone();
            let handle = spawn_blocking(move || {
                SetAchievement {
                    app_id,
                    achievement_id,
                    unlocked
                }.request()
            });
            MainContext::default().spawn_local(clone!(#[weak] switch, async move {
                if Some(Some(true)) != handle.await.ok() {
                    switch.set_active(!switch.is_active());
                }
                switch.set_sensitive(true);
            })); 
        }));

        let switch_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::End)
            .build();
        switch_box.append(&switch);
        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let name_label = Label::builder()
            .label(achievement.name())
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let description_label = Label::builder()
            .label(achievement.description())
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let label_box = Box::builder()
            .orientation(Orientation::Vertical)
            .build();
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
        achievement_box.into()
    });

    (app_achievements_list, app_achievements_model, app_achievement_string_filter)
}