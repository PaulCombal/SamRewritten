use std::cell::Cell;
use std::rc::Rc;
use gtk::gio::{spawn_blocking, ListStore};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{glib, Align, Box, ClosureExpression, FilterListModel, Label, ListItem, ListView, NoSelection, Orientation, SignalListItemFactory, Stack, StackTransitionType, StringFilter, StringFilterMatchMode, Switch, Widget};
use gtk::glib::{clone, MainContext};
use crate::frontend::achievement::GAchievementObject;
use crate::frontend::request::{Request, SetAchievement};
use crate::frontend::shimmer_image::ShimmerImage;

pub fn create_achievements_view(app_id: Rc<Cell<Option<u32>>>) -> (ListView, ListStore, StringFilter) {
    let achievements_list_factory = SignalListItemFactory::new();
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
    let app_achievement_selection_model = NoSelection::new(Option::<ListStore>::None);
    app_achievement_selection_model.set_model(Some(&app_achievement_filter_model));

    let app_achievements_list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(&app_achievement_selection_model)
        .factory(&achievements_list_factory)
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

        let switch = Switch::builder()
            .valign(Align::Center)
            .build();

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
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let description_label = Label::builder()
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

        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        list_item.set_child(Some(&achievement_box));

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
            .chain_property::<GAchievementObject>("icon_normal")
            .bind(&normal_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon_locked")
            .bind(&locked_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is_achieved")
            .bind(&switch, "active", Widget::NONE);
        
        // Custom expressions
        let is_achieved_expr = list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is_achieved");
        let rust_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let is_achieved = values.get(1)
                .and_then(|val| val.get::<bool>().ok())
                .unwrap_or(false);
            let child_name = if is_achieved { "normal" } else { "locked" };
            Some(child_name.to_value())
        });

        let visible_child_expr = ClosureExpression::new::<String>(
            &[is_achieved_expr], rust_closure
        );

        visible_child_expr.bind(&icon_stack, "visible-child-name", Widget::NONE);
    });

    achievements_list_factory.connect_bind(move |_, list_item| unsafe {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        let achievement_object = list_item.item()
            .and_then(|item| item.downcast::<GAchievementObject>().ok())
            .expect("Item should be a GAchievementObject");

        let switch = list_item.child()
            .and_then(|child| child.downcast::<Box>().ok())
            .and_then(|box_widget| box_widget.last_child()) // Assuming switch_box is the last child
            .and_then(|last_child| last_child.downcast::<Box>().ok()) // switch_box
            .and_then(|switch_box| switch_box.last_child()) // switch
            .and_then(|switch_widget| switch_widget.downcast::<Switch>().ok())
            .expect("Could not find Switch widget");

        // Disconnect previous handler if it exists
        if let Some(handler_id) = list_item.steal_data::<glib::SignalHandlerId>("switch-state-notify-handler") {
            switch.disconnect(handler_id);
        }

        let app_id = app_id.get().unwrap_or_default();
        let achievement_id = achievement_object.id().clone();

        switch.connect_state_notify(move |switch| {
            if !switch.is_sensitive() { return }
            switch.set_sensitive(false);
            let unlocked = switch.is_active();
            achievement_object.set_is_achieved(unlocked);
            let achievement_id = achievement_id.clone();
            let handle = spawn_blocking(move || {
                SetAchievement {
                    app_id,
                    achievement_id,
                    unlocked
                }.request()
            });
            MainContext::default().spawn_local(clone!(
                #[weak] switch,
                #[weak] achievement_object,
                async move {
                    if Some(Some(true)) != handle.await.ok() {
                        achievement_object.set_is_achieved(!unlocked);
                    }
                    switch.set_sensitive(true);
                }
            ));
        });
    });

    achievements_list_factory.connect_unbind(move |_, list_item| unsafe {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        let switch = list_item.child()
            .and_then(|child| child.downcast::<Box>().ok())
            .and_then(|box_widget| box_widget.last_child()) // Assuming switch_box is the last child
            .and_then(|last_child| last_child.downcast::<Box>().ok()) // switch_box
            .and_then(|switch_box| switch_box.last_child()) // switch
            .and_then(|switch_widget| switch_widget.downcast::<Switch>().ok())
            .expect("Could not find Switch widget");

        // Disconnect handler when item is unbound
        if let Some(handler_id) = list_item.steal_data::<glib::SignalHandlerId>("switch-state-notify-handler") {
            switch.disconnect(handler_id);
        }
    });

    (app_achievements_list_view, app_achievements_model, app_achievement_string_filter)
}