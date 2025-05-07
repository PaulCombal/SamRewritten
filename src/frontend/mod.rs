pub mod ipc_process;
mod request;
mod shimmer_image;
mod steam_app;

use crate::{APP_ID, dev_println};
use crate::frontend::request::GetOwnedAppList;
use std::cell::{Cell, RefCell};
use std::process::Child;
use std::rc::Rc;
use gtk::pango::WrapMode;
use request::{GetAchievements, GetStats, LaunchApp, Request, Shutdown, StopApp};
use shimmer_image::ShimmerImage;
use steam_app::GSteamAppObject;
use gdk::prelude::*;
use gtk::gio::spawn_blocking;
use gtk::glib::{clone, ExitCode, MainContext};
use gtk::prelude::{EditableExt, GObjectPropertyExpressionExt, ToggleButtonExt};
use gtk::prelude::{ApplicationExt, BoxExt, ButtonExt, Cast, GtkWindowExt, ListItemExt, WidgetExt};
use gtk::{Align, FilterListModel, Frame, Paned, SearchEntry, Separator, StringFilter, StringFilterMatchMode, ToggleButton, Widget};
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

    let app_achievements_frame = Frame::builder()
        .label("Achievements")
        .margin_top(20)
        .margin_start(20)
        .height_request(2000)
        .margin_end(20)
        .margin_bottom(20)
        .build();
    let app_achievements_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .child(&app_achievements_frame)
        .build(); 

    let app_stats_frame = Frame::builder()
        .label("Stats")
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
    let list_spinner_box = Box::builder().halign(gtk::Align::Center).build();
    list_spinner_box.append(&list_spinner);
    list_spinner_box.append(&list_spinner_label);

    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    let search_entry = SearchEntry::builder().placeholder_text("App name or App ID..").build();
    let back_button = Button::builder().icon_name("go-previous").sensitive(false).build();
    let refresh_button = Button::builder().icon_name("view-refresh").sensitive(false).build();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&refresh_button); 

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
        #[weak] app_achievement_count_value,
        #[weak] app_stats_count_value,
        #[weak] app_type_value,
        #[weak] app_stack,
        #[weak] list_stack, move |list_view, position| {
        let Some(model) = list_view.model() else { return };
        let Some(item) = model.item(position).and_downcast::<GSteamAppObject>() else { return };
        app_type_value.set_label("Loading...");
        app_achievement_count_value.set_label("Loading...");
        app_stats_count_value.set_label("Loading...");
        app_stack.set_visible_child_name("loading");
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
            app_type_value.set_label(&format!("{app_type_copy}"));
            app_achievement_count_value.set_label(&format!("{completed}/{}", achievements.len()));
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
        #[weak] back_button, #[weak] refresh_button, move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                back_button.set_sensitive(false);
                refresh_button.set_sensitive(false);
            } else if stack.visible_child_name().as_deref() == Some("app") {
                back_button.set_sensitive(true);
                refresh_button.set_sensitive(false);
            } else {
                back_button.set_sensitive(false);
                refresh_button.set_sensitive(true);
            }
        }
    ));

    search_entry.connect_search_changed(clone!(
        #[weak] list_string_filter, move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());
            list_string_filter.set_search(text.as_deref()); 
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