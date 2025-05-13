pub mod ipc_process;
mod request;
mod shimmer_image;
mod steam_app;
mod achievement;

use crate::{APP_ID, dev_println};
use crate::frontend::request::GetOwnedAppList;
use std::cell::{Cell, RefCell};
use std::io::Cursor;
use std::process::Child;
use std::rc::Rc;
use achievement::GAchievementObject;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::pango::WrapMode;
use request::{GetAchievements, GetStats, LaunchApp, Request, SetAchievement, Shutdown, StopApp};
use shimmer_image::ShimmerImage;
use steam_app::GSteamAppObject;
use gdk::prelude::*;
use gtk::gio::spawn_blocking;
use gtk::glib::{clone, ExitCode, MainContext};
use gtk::prelude::{EditableExt, GObjectPropertyExpressionExt, ToggleButtonExt};
use gtk::prelude::{ApplicationExt, BoxExt, ButtonExt, Cast, GtkWindowExt, ListItemExt, WidgetExt};
use gtk::{AboutDialog, Align, FilterListModel, Frame, License, ListBox, Paned, SearchEntry, SelectionMode, Separator, StringFilter, StringFilterMatchMode, Switch, ToggleButton, Widget};
use gtk::{
    glib, gdk, gio::ListStore, Application, ApplicationWindow, Box, Button, HeaderBar, Image,
    Label, ListItem, ListView, NoSelection, Orientation, PolicyType, ScrolledWindow,
    SignalListItemFactory, Spinner, Stack, StackTransitionType
};

fn activate(application: &Application) {

    //Creating application view
    let app_id = Rc::new(Cell::new(Option::<u32>::None));
    let app_spinner = Spinner::builder().spinning(true).margin_end(5).build();
    let app_spinner_label = Label::builder().label("Loading...").build();
    let app_spinner_box = Box::builder().halign(Align::Center).build();
    app_spinner_box.append(&app_spinner);
    app_spinner_box.append(&app_spinner_label);

    let app_achievement_count_label = Label::builder().label("Achievements:").halign(Align::Start).build();
    let app_achievement_count_spacer = Box::builder().hexpand(true).build();
    let app_achievement_count_value = Label::builder().halign(Align::End).build();
    let app_achievement_count_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_achievement_count_box.append(&app_achievement_count_label);
    app_achievement_count_box.append(&app_achievement_count_spacer);
    app_achievement_count_box.append(&app_achievement_count_value);

    let app_stats_count_label = Label::builder().label("Stats:").halign(Align::Start).build();
    let app_stats_count_spacer = Box::builder().hexpand(true).build();
    let app_stats_count_value = Label::builder().halign(Align::End).build();
    let app_stats_count_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_stats_count_box.append(&app_stats_count_label);
    app_stats_count_box.append(&app_stats_count_spacer);
    app_stats_count_box.append(&app_stats_count_value);

    let app_type_label = Label::builder().label("Type:").halign(Align::Start).build();
    let app_type_spacer = Box::builder().hexpand(true).build();
    let app_type_value = Label::builder().halign(Align::End).build();
    let app_type_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(20)
        .build();
    app_type_box.append(&app_type_label);
    app_type_box.append(&app_type_spacer);
    app_type_box.append(&app_type_value);

    let app_loading_failed_label = Label::builder()
        .label("Failed to load app.")
        .halign(Align::Center)
        .valign(Align::Center)
        .build(); 

    let app_label = Label::builder()
        .margin_top(20)
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(Align::Start)
        .build();
    let app_shimmer_image = ShimmerImage::new();
    app_shimmer_image.set_halign(Align::Start);
    let app_achievements_button = ToggleButton::builder().label("Achievements").build();
    let app_stats_button = ToggleButton::builder().label("Stats").group(&app_achievements_button).build();
    let app_button_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"].as_slice())
        .margin_bottom(20)
        .margin_start(0)
        .homogeneous(true)
        .margin_end(0)
        .width_request(231)
        .halign(Align::Start)
        .build();
    app_button_box.append(&app_achievements_button);
    app_button_box.append(&app_stats_button);
 
    let app_sidebar_separator = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(20)
        .build(); 
    let app_sidebar = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();
    app_sidebar.append(&app_button_box);
    app_sidebar.append(&app_shimmer_image);
    app_sidebar.append(&app_label);
    app_sidebar.append(&app_sidebar_separator);
    app_sidebar.append(&app_type_box);
    app_sidebar.append(&app_achievement_count_box);
    app_sidebar.append(&app_stats_count_box);

    let app_achievements_model = ListStore::new::<GAchievementObject>();
    let app_achievement_string_filter = StringFilter::builder()
        .expression(&GAchievementObject::this_expression("description"))
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
            .halign(Align::Start)
            .build();
        let description_label = Label::builder()
            .label(achievement.description())
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

    let app_achievements_frame = Frame::builder()
        .child(&app_achievements_list)
        .build();
    let app_achievements_spacer = Box::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .build();
    let app_achievement_box = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_start(20)
        .margin_end(20)
        .margin_bottom(20)
        .build();
    app_achievement_box.append(&app_achievements_frame);
    app_achievement_box.append(&app_achievements_spacer);
    let app_achievements_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .child(&app_achievement_box)
        .build(); 

    let app_stats_frame = Frame::builder()
        .margin_top(20)
        .margin_start(20)
        .margin_end(20)
        .margin_bottom(20)
        .build();
    let app_stats_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .child(&app_stats_frame)
        .build(); 

    let app_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build(); 
    app_stack.add_named(&app_achievements_scrolled_window, Some("achievements"));
    app_stack.add_named(&app_loading_failed_label, Some("failed"));
    app_stack.add_named(&app_spinner_box, Some("loading"));
    app_stack.add_named(&app_stats_scrolled_window, Some("stats"));

    app_stack.connect_visible_child_name_notify(clone!(
        #[weak] app_achievements_button, #[weak] app_stats_button, move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                app_achievements_button.set_sensitive(false);
                app_stats_button.set_sensitive(false);
            } else if stack.visible_child_name().as_deref() == Some("failed") {
                app_achievements_button.set_sensitive(false);
                app_stats_button.set_sensitive(false); 
            } else if stack.visible_child_name().as_deref() == Some("achievements") {
                app_achievements_button.set_active(true);
                app_stats_button.set_active(false);
                app_achievements_button.set_sensitive(true);
                app_stats_button.set_sensitive(true);
            } else {
                app_achievements_button.set_active(false);
                app_stats_button.set_active(true);
                app_achievements_button.set_sensitive(true);
                app_stats_button.set_sensitive(true);
            }
        }
    )); 

    let app_pane = Paned::builder()
        .orientation(Orientation::Horizontal)
        .shrink_start_child(false)
        .shrink_end_child(false)
        .resize_start_child(false) 
        .start_child(&app_sidebar)
        .end_child(&app_stack)
        .build();

    //Creating application list view
    let list_spinner = Spinner::builder().margin_end(5).spinning(true).build();
    let list_spinner_label = Label::builder().label("Loading...").build();
    let list_spinner_box = Box::builder().halign(Align::Center).build();
    list_spinner_box.append(&list_spinner);
    list_spinner_box.append(&list_spinner_label);

    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    let search_entry = SearchEntry::builder().placeholder_text("App name").build();
    let back_button = Button::builder().icon_name("go-previous").sensitive(false).build();
    let refresh_button = Button::builder().icon_name("view-refresh").sensitive(false).build();
    let context_menu_button = Button::builder().icon_name("open-menu-symbolic").build();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&context_menu_button);
    header_bar.pack_end(&refresh_button);

    #[cfg(target_os = "windows")]
    let image_bytes = include_bytes!("..\\..\\assets\\icon_256.png");
    #[cfg(target_os = "linux")]
    let image_bytes = include_bytes!("../../assets/icon_256.png");
    let logo_pixbuf = Pixbuf::from_read(Cursor::new(image_bytes)).expect("Failed to load logo");
    let logo = Image::from_pixbuf(Some(&logo_pixbuf)).paintable().expect("Failed to create logo image");

    let about_dialog = AboutDialog::builder()
        .version(env!("CARGO_PKG_VERSION"))
        .license_type(License::Gpl30)
        .program_name("SamRewritten 2")
        .authors(env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<_>>())
        .comments(env!("CARGO_PKG_DESCRIPTION"))
        .logo(&logo)
        .build(); 

    context_menu_button.connect_clicked(clone!(#[weak] about_dialog, move |_| {
        about_dialog.show();
    }));

    let list_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .build();
    let list_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    list_stack.add_named(&list_spinner_box, Some("loading"));
    list_stack.add_named(&list_scrolled_window, Some("list"));
    list_stack.add_named(&app_pane, Some("app"));

    let list_factory = SignalListItemFactory::new(); 
    let list_store = ListStore::new::<GSteamAppObject>();
    let list_string_filter = StringFilter::builder()
        .expression(&GSteamAppObject::this_expression("app_name"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let list_filter_model = FilterListModel::builder()
        .model(&list_store)
        .filter(&list_string_filter)
        .build();
    let list_selection_model = NoSelection::new(Option::<ListStore>::None);
    list_selection_model.set_model(Some(&list_filter_model));
    let list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(&list_selection_model)
        .factory(&list_factory)
        .build();

    let window = ApplicationWindow::builder()
        .application(application)
        .title("SamRewritten")
        .default_width(800)
        .default_height(600)
        .child(&list_stack)
        .build();
    window.set_titlebar(Some(&header_bar)); 

    list_view.connect_activate(clone!(
        #[strong] app_id,
        #[weak] app_achievements_model,
        #[weak] app_achievement_count_value,
        #[weak] app_stats_count_value,
        #[weak] app_type_value,
        #[weak] app_stack,
        #[weak] list_stack, move |list_view, position| {
        let Some(model) = list_view.model() else { return };
        let Some(item) = model.item(position).and_downcast::<GSteamAppObject>() else { return };
        app_type_value.set_label("...");
        app_achievement_count_value.set_label("...");
        app_stats_count_value.set_label("...");
        app_stack.set_visible_child_name("loading");
        app_achievements_model.remove_all();
        app_id.set(Some(item.app_id())); 

        let app_type_copy = item.app_type();
        let app_id_copy = item.app_id();
        let handle = spawn_blocking(move || {
            LaunchApp { app_id: app_id_copy }.request();
            let achievements = GetAchievements { app_id: app_id_copy }.request();
            let stats = GetStats { app_id: app_id_copy }.request();
            let completed_achievements = achievements.as_deref()
                .unwrap_or_default()
                .iter().filter(|a| a.is_achieved)
                .count();
            (achievements, stats, completed_achievements)
        });
        MainContext::default().spawn_local(clone!(async move {
            let Ok((Some(achievements), Some(stats), completed)) = handle.await else {
                return app_stack.set_visible_child_name("failed");
            };

            app_achievement_count_value.set_label(&format!("{completed}/{}", achievements.len()));
            achievements.into_iter().map(GAchievementObject::new)
                .for_each(|achievement| app_achievements_model.append(&achievement));
            app_type_value.set_label(&format!("{app_type_copy}"));
            app_stats_count_value.set_label(&format!("{}", stats.len()));
            app_stack.set_visible_child_name("achievements");
        }));

        if let Some(url) = item.image_url() { app_shimmer_image.set_url(url.as_str()); }
        else { app_shimmer_image.reset(); }
        app_label.set_markup(&format!("<span font_desc=\"Bold 16\">{}</span>", item.app_name()));
        list_stack.set_visible_child_name("app");
    }));

    list_factory.connect_setup(clone!(#[weak] list_view, move |_, list_item| {
        let image = ShimmerImage::new();
        let label = Label::builder().margin_start(20).build();
        let spacer = Box::builder().orientation(Orientation::Horizontal).hexpand(true).build();
        let icon = Image::builder().icon_name("pan-end").margin_end(20).build();
        let entry = Box::builder().orientation(Orientation::Horizontal).margin_top(4).margin_bottom(4).build();
        let button = Button::builder().child(&entry).margin_start(4).margin_end(4).margin_top(4).build();
        entry.append(&image);
        entry.append(&label);
        entry.append(&spacer);
        entry.append(&icon);

        let list_item = list_item.downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        list_item.set_child(Some(&button));
        list_item.property_expression("item")
            .chain_property::<GSteamAppObject>("app_name")
            .bind(&label, "label", Widget::NONE); 
        list_item.property_expression("item")
            .chain_property::<GSteamAppObject>("image_url")
            .bind(&image, "url", Widget::NONE);

        button.connect_clicked(clone!(#[weak] list_item, #[weak] list_view, move |_| {
            list_view.emit_by_name::<()>("activate", &[&list_item.position()]); 
        })); 
    }));

    refresh_button.connect_clicked(clone!(
        #[strong] list_view,
        #[strong] list_store,
        #[weak] list_scrolled_window,
        #[weak] list_stack,
        move |button| {
            button.set_sensitive(false);
            list_stack.set_visible_child_name("loading");
            let apps = spawn_blocking(move || GetOwnedAppList.request()); 
            MainContext::default().spawn_local(clone!(
                #[weak] list_view,
                #[weak] list_scrolled_window,
                #[weak] list_store,
                #[weak] list_stack,
                async move {
                    if let Some(apps) = apps.await.ok().flatten() {
                        if apps.is_empty() {
                            let label = Label::new(Some("No apps found."));
                            list_scrolled_window.set_child(Some(&label));
                            list_stack.set_visible_child_name("list");
                        } else {
                            list_store.remove_all();
                            apps.into_iter().map(GSteamAppObject::new)
                                .for_each(|app| list_store.append(&app));
                            list_scrolled_window.set_child(Some(&list_view));
                            list_stack.set_visible_child_name("list");
                        }
                    } else {
                        let label = Label::new(Some("Failed to load apps."));
                        list_scrolled_window.set_child(Some(&label));
                        list_stack.set_visible_child_name("list");
                    }
                }
            ));
        }
    ));

    list_stack.connect_visible_child_notify(clone!(
        #[weak] back_button, #[weak] search_entry, #[weak] refresh_button, move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                back_button.set_sensitive(false);
                refresh_button.set_sensitive(false);
            } else if stack.visible_child_name().as_deref() == Some("app") {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("Achievement name..."));
                back_button.set_sensitive(true);
                refresh_button.set_sensitive(false);
            } else {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("App name..."));
                back_button.set_sensitive(false);
                refresh_button.set_sensitive(true);
            }
        }
    ));

    search_entry.connect_search_changed(clone!(
        #[weak] list_string_filter, #[weak] list_stack, move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());
            if list_stack.visible_child_name().as_deref() == Some("app") {
                app_achievement_string_filter.set_search(text.as_deref());
            } else {
                list_string_filter.set_search(text.as_deref()); 
            }
        }
    ));

    back_button.connect_clicked(clone!(
        #[weak] list_stack, #[weak] app_id, move |_| {
            list_stack.set_visible_child_name("list");
            if let Some(app_id) = app_id.take() {
                spawn_blocking(move || {
                    StopApp { app_id }.request();
                });
            }
        }
    ));

    app_achievements_button.connect_clicked(clone!(
        #[weak] app_stack, move |_| {
            app_stack.set_visible_child_name("achievements");
        }
    ));

    app_stats_button.connect_clicked(clone!(
        #[weak] app_stack, move |_| {
            app_stack.set_visible_child_name("stats");
        }
    ));

    app_stack.set_visible_child_name("loading");
    list_stack.set_visible_child_name("loading");
    refresh_button.emit_clicked();
    window.present();
}

fn shutdown(orchestrator: &RefCell<Child>) {
    Shutdown.request();

    match orchestrator.borrow_mut().wait() {
        Ok(code) => dev_println!("[CLIENT] Orchestrator process exited with: {code}"),
        Err(error) => dev_println!("[CLIENT] Failed to wait for orchestrator process: {error}"),
    }
}

pub fn main_ui(orchestrator: Child) -> ExitCode {
    let orchestrator = RefCell::new(orchestrator);
    let main_app = gtk::Application::builder().application_id(APP_ID).build();

    main_app.connect_activate(activate);
    main_app.connect_shutdown(move |_| shutdown(&orchestrator));
    main_app.run()
}