pub mod ipc_process;
mod request;
mod shimmer_image;
mod steam_app;

use crate::{APP_ID, dev_println};
use crate::frontend::request::GetOwnedAppList;
use std::cell::RefCell;
use std::process::Child;
use request::{Request, Shutdown};
use shimmer_image::ShimmerImage;
use steam_app::GSteamAppObject;
use gdk::prelude::*;
use gtk::gio::spawn_blocking;
use gtk::glib::{clone, ExitCode, MainContext};
use gtk::prelude::{EditableExt, GObjectPropertyExpressionExt};
use gtk::prelude::{ApplicationExt, BoxExt, ButtonExt, Cast, GtkWindowExt, ListItemExt, WidgetExt};
use gtk::{FilterListModel, SearchEntry, StringFilter, StringFilterMatchMode, Widget};
use gtk::{
    glib, gdk, gio::ListStore, Application, ApplicationWindow, Box, Button, HeaderBar, Image,
    Label, ListItem, ListView, NoSelection, Orientation, PolicyType, ScrolledWindow,
    SignalListItemFactory, Spinner, Stack, StackTransitionType
};


fn activate(application: &Application) {
    let spinner = Spinner::builder().margin_end(5).spinning(true).build();
    let spinner_label = Label::builder().label("Loading...").build();
    let spinner_box = Box::builder().halign(gtk::Align::Center).build();
    spinner_box.append(&spinner);
    spinner_box.append(&spinner_label);

    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    let search_entry = SearchEntry::builder().placeholder_text("App name or App ID..").build();
    let back_button = Button::builder().icon_name("go-previous").sensitive(false).build();
    let refresh_button = Button::builder().icon_name("view-refresh").sensitive(false).build();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&refresh_button); 

    let achievements = Label::new(Some("Achievement list here"));
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .build();
    let stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    stack.add_named(&spinner_box, Some("loading"));
    stack.add_named(&scrolled_window, Some("picker"));
    stack.add_named(&achievements, Some("app"));

    let window = ApplicationWindow::builder()
        .application(application)
        .title("SamRewritten")
        .default_width(800)
        .default_height(600)
        .child(&stack)
        .build();
    window.set_titlebar(Some(&header_bar)); 

    let factory = SignalListItemFactory::new(); 
    let list_store = ListStore::new::<GSteamAppObject>();
    let string_filter = StringFilter::builder()
        .expression(&GSteamAppObject::this_expression("app_name"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let filter_list_model = FilterListModel::builder()
        .model(&list_store)
        .filter(&string_filter)
        .build();
    let selection_model = NoSelection::new(Option::<ListStore>::None);
    selection_model.set_model(Some(&filter_list_model));
    let list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(&selection_model)
        .factory(&factory)
        .build();

    factory.connect_setup(clone!(#[weak] stack, move |_, list_item| {
        let image = ShimmerImage::new();
        let label = Label::builder().margin_start(20).build();
        let spacer = Box::builder().orientation(Orientation::Horizontal).hexpand(true).build();
        let icon = Image::builder().icon_name("pan-end").margin_end(20).build();
        let entry = Box::builder().orientation(Orientation::Horizontal).margin_top(4).margin_bottom(4).build();
        let button = Button::builder().child(&entry).margin_start(4).margin_end(4).margin_top(4).build();
        button.connect_clicked(clone!(#[weak] stack, move |_| stack.set_visible_child_name("app")));

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
    }));

    refresh_button.connect_clicked(clone!(
        #[strong] list_store,
        #[weak] scrolled_window,
        #[weak] list_view,
        #[weak] stack,
        move |button| {
            button.set_sensitive(false);
            stack.set_visible_child_name("loading");
            let apps = spawn_blocking(move || GetOwnedAppList.request()); 
            MainContext::default().spawn_local(clone!(
                #[strong] list_view,
                #[weak] scrolled_window,
                #[weak] list_store,
                #[weak] stack,
                async move {
                    if let Some(apps) = apps.await.ok().flatten() {
                        if apps.is_empty() {
                            let label = Label::new(Some("No apps found."));
                            scrolled_window.set_child(Some(&label));
                            stack.set_visible_child_name("picker");
                        } else {
                            list_store.remove_all();
                            apps.into_iter().map(GSteamAppObject::new)
                                .for_each(|app| list_store.append(&app));
                            scrolled_window.set_child(Some(&list_view));
                            stack.set_visible_child_name("picker");
                        }
                    } else {
                        let label = Label::new(Some("Failed to load apps."));
                        scrolled_window.set_child(Some(&label));
                        stack.set_visible_child_name("picker");
                    }
                }
            ));
        }
    ));

    stack.connect_visible_child_notify(clone!(
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
        #[weak] string_filter, move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());
            string_filter.set_search(text.as_deref()); 
        }
    ));

    back_button.connect_clicked(clone!(
        #[weak] stack, move |_| stack.set_visible_child_name("picker")
    ));

    stack.set_visible_child_name("loading");
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