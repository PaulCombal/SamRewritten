use crate::backend::app_lister::AppModel;
use crate::frontend::gtk_wrappers;
use crate::frontend::request::{
    GetAchievements, GetOwnedAppList, GetStats, LaunchApp, Request, StopApps,
};
use crate::{APP_ID, dev_println};
use gdk::prelude::*;
use gtk::glib::{ExitCode, clone};
use gtk::prelude::EditableExt;
use gtk::prelude::{
    ApplicationExt, BoxExt, ButtonExt, Cast, CastNone, GtkWindowExt, ListItemExt, WidgetExt,
};
use gtk::{
    Application, SignalListItemFactory, gdk, gio, glib, style_context_add_provider_for_display,
};
use std::cell::RefCell;
use std::env;
use std::process::Child;

use super::request::Shutdown;

// GTK / UI
// TODO: Add an empty widget when there are no apps to show
// TODO: Show an error widget when the connection to steam failed
fn activate(app: &Application) {
    // --- CSS ---
    let provider = gtk::CssProvider::new();
    provider.load_from_data(".rounded-image { border-radius: 10px; overflow: hidden; }");

    if let Some(display) = gtk::gdk::Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Async message channels
    let (m_picker_apps_sender, m_picker_apps_receiver) =
        async_channel::bounded::<Option<Vec<AppModel>>>(1);

    // Create new model
    let picker_model = gio::ListStore::new::<gtk_wrappers::GSteamAppObject>();
    let picker_model_clone = picker_model.clone();

    let picker_string_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
        gtk_wrappers::GSteamAppObject::static_type(),
        None::<gtk::Expression>,
        "app_name",
    )));
    picker_string_filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
    picker_string_filter.set_ignore_case(true);
    let picker_string_filter_clone = picker_string_filter.clone();

    let picker_filter_model =
        gtk::FilterListModel::new(Some(picker_model), Some(picker_string_filter));

    let app_ach_model = gio::ListStore::new::<gtk_wrappers::GSteamAppObject>();

    let picker_factory = SignalListItemFactory::new();
    let app_ach_factory = SignalListItemFactory::new();

    picker_factory.connect_setup(move |_, list_item| {
        let app_list_item_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let picture = gtk::Picture::new();
        let label = gtk::Label::new(None);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let icon = gtk::Image::from_icon_name("pan-end");

        // picture.set_size_request(231, 87); // Native size
        picture.set_size_request(162, 61);
        picture.set_margin_top(5);
        picture.set_margin_bottom(5);
        picture.set_margin_start(5);
        picture.set_overflow(gtk::Overflow::Hidden);
        picture.add_css_class("rounded-image");

        label.set_margin_start(10);
        spacer.set_hexpand(true);
        icon.set_margin_end(10);

        app_list_item_box.append(&picture);
        app_list_item_box.append(&label);
        app_list_item_box.append(&spacer);
        app_list_item_box.append(&icon);

        list_item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&app_list_item_box));
    });

    app_ach_factory.connect_setup(move |_, list_item| {
        let ach_list_item_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let picture = gtk::Picture::new();
        let labels_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let label_title = gtk::Label::new(None);
        let label_desc = gtk::Label::new(None);
        let switch = gtk::Switch::new();

        picture.set_size_request(64, 64);
        picture.set_margin_top(5);
        picture.set_margin_bottom(5);
        picture.set_margin_start(5);
        picture.set_overflow(gtk::Overflow::Hidden);
        labels_box.append(&label_title);
        labels_box.append(&label_desc);
        labels_box.set_hexpand(true);

        ach_list_item_box.append(&picture);
        ach_list_item_box.append(&labels_box);
        ach_list_item_box.append(&switch);

        list_item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&ach_list_item_box));
    });

    // This is extremely unoptimized.
    // A gtk/Rust guru is needed to fix this piece of shit code
    // Scroll at the middle of the list: once there, any slight movement, up or down
    // will trigger the image load of hundreds of apps. This is extremely I/O costing.
    // Sometimes, pictures of elements on screen are refreshed with the wrong picture, which should not happen.
    // It looks like GTK has no fucking clue which list items are on screen.
    picker_factory.connect_bind(move |_, list_item| {
        let steam_app_object = list_item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<gtk_wrappers::GSteamAppObject>()
            .expect("The item has to be an `GSteamAppObject`.");

        let app_row_box = list_item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<gtk::Box>()
            .expect("The child has to be a `Box`.");

        let picture_widget = app_row_box.first_child().expect("No child");
        let picture = picture_widget
            .downcast_ref::<gtk::Picture>()
            .expect("Needs to be Picture");

        let label_widget = picture_widget // Use the widget, not the borrowed `picture` reference
            .next_sibling()
            .expect("No child");
        let label = label_widget
            .downcast_ref::<gtk::Label>()
            .expect("Needs to be Label");

        // --- Start Image Loading ---
        picture.set_paintable(None::<&gdk::Paintable>);
        let app_id = steam_app_object.app_id();
        let app_name = steam_app_object.app_name();

        if let Some(image_url) = steam_app_object.image_url() {
            dev_println!(
                "[CLIENT] Loading image for app {}",
                steam_app_object.app_name()
            );
            let image_url_clone = image_url.clone();
            let picture_weak = picture.downgrade();

            glib::spawn_future_local(async move {
                // if let Some(p) = picture_weak.upgrade() { p.set_icon_name(Some("image-loading-symbolic")); }

                let image_data_result = gio::spawn_blocking(move || {
                    let tmp_path = env::temp_dir().join(format!("sam2_app_img_{}.jpg", app_id));

                    // Try to load from cache first
                    if tmp_path.exists() {
                        match std::fs::read(&tmp_path) {
                            Ok(bytes) => return Ok(bytes),
                            Err(e) => eprintln!("[CLIENT] Failed to read cached image: {}", e),
                        }
                    }

                    dev_println!("[CLIENT] Downloading image for app {}", app_id);

                    // Download if not cached
                    let client = reqwest::blocking::Client::new();
                    match client.get(&image_url).send() {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.bytes() {
                                    Ok(bytes) => {
                                        if let Err(e) = std::fs::write(&tmp_path, &bytes) {
                                            eprintln!(
                                                "[CLIENT] Failed to save image to {}: {}",
                                                tmp_path.display(),
                                                e
                                            );
                                        }

                                        Ok(Vec::from(bytes))
                                    }
                                    Err(e) => Err(format!("Failed to read image bytes: {}", e)),
                                }
                            } else {
                                Err(format!("HTTP error: {}", response.status()))
                            }
                        }
                        Err(e) => Err(format!("Request failed: {}", e)),
                    }
                })
                .await;

                // TODO: Fix the wrong app picture issue
                // We just downloaded the image in an async block. This fucking sucks because now
                // the list view has potentially reassigned the picture widget to another list item.
                // We need to look again in the list if the app is still loaded and then assign the image.
                // If not, then just forget about it.
                // A frontend engineer needs to put some order in this mess.

                if let Some(picture) = picture_weak.upgrade() {
                    match image_data_result {
                        Ok(Ok(image_bytes)) => {
                            let gbytes = glib::Bytes::from(&image_bytes);
                            match gdk::Texture::from_bytes(&gbytes) {
                                Ok(texture) => {
                                    picture.set_paintable(Some(&texture));
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[CLIENT] Failed to create texture for {}: {}",
                                        image_url_clone, e
                                    );
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            dev_println!(
                                "[CLIENT] Failed to fetch or process image {}: {}",
                                image_url_clone,
                                e
                            );
                        }
                        Err(e) => {
                            dev_println!(
                                "[CLIENT] Error in spawn_blocking for image fetch {}: {:?}",
                                image_url_clone,
                                e
                            );
                        }
                    }
                } else {
                    dev_println!(
                        "[CLIENT] Picture widget was dropped before image {} could be set.",
                        image_url_clone
                    );
                }
            });
        } else {
            dev_println!(
                "[CLIENT] No image URL for app {}",
                steam_app_object.app_name()
            );
            picture.set_paintable(None::<&gdk::Paintable>);
        }

        // --- End Image Loading ---

        label.set_label(&steam_app_object.app_name());
    });

    let picker_selection_model = gtk::NoSelection::new(Some(picker_filter_model));

    // Instantiate all widgets at the application startup
    let w_picker_list_view = gtk::ListView::new(Some(picker_selection_model), Some(picker_factory));
    let w_spinner = gtk::Spinner::new();
    let w_picker_main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let w_picker_loading_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let w_picker_scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(360)
        .child(&w_picker_list_view)
        .build();

    let w_header = gtk::HeaderBar::builder().show_title_buttons(true).build();

    let w_header_bar_search_bar = gtk::SearchEntry::builder()
        .placeholder_text("App name or App ID..")
        .build();

    let w_header_bar_back_button = gtk::Button::builder()
        .icon_name("go-previous")
        .width_request(40)
        .visible(false)
        .build();

    let w_header_bar_refresh_button = gtk::Button::with_label("Refresh");
    let w_header_bar_refresh_button_clone = w_header_bar_refresh_button.clone();
    let w_placeholder = gtk::Label::new(Some("Achievements list here"));
    let w_main_stack = gtk::Stack::new();

    let w_window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SamRewritten")
        .default_width(800)
        .default_height(600)
        .child(&w_main_stack)
        .build();

    w_main_stack.add_named(&w_picker_main_box, Some("picker"));
    w_main_stack.add_named(&w_placeholder, Some("app"));
    w_main_stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    w_window.set_titlebar(Some(&w_header));
    w_picker_scrolled_window.set_vexpand(true);
    w_picker_loading_box.set_halign(gtk::Align::Center);
    w_picker_loading_box.set_valign(gtk::Align::Center);
    w_picker_loading_box.set_hexpand(true);
    w_picker_loading_box.set_vexpand(true);
    w_picker_loading_box.append(&w_spinner);
    w_picker_main_box.append(&w_picker_loading_box);
    w_spinner.start();
    w_spinner.set_size_request(48, 48);
    w_header.pack_start(&w_header_bar_back_button);
    w_header.pack_start(&w_header_bar_search_bar);
    w_header.pack_end(&w_header_bar_refresh_button);

    // let w_window_clone = w_window.clone();
    let w_picker_loading_box_clone = w_picker_loading_box.clone();
    let w_picker_scrolled_window_clone = w_picker_scrolled_window.clone();
    let w_main_stack_clone = w_main_stack.clone();
    let w_main_stack_clone2 = w_main_stack.clone();
    let w_main_stack_clone3 = w_main_stack.clone();
    let w_header_bar_back_button_clone = w_header_bar_back_button.clone();
    let w_header_bar_back_button_clone2 = w_header_bar_back_button.clone();

    w_header_bar_search_bar.connect_changed(glib::clone!(
        #[weak]
        picker_string_filter_clone,
        move |entry| {
            let entry_text = entry.text();
            let search_text = if entry_text.is_empty() {
                None
            } else {
                Some(entry_text.as_str())
            };
            picker_string_filter_clone.set_search(search_text);
        }
    ));

    w_header_bar_refresh_button.connect_clicked(move |_| {
        dev_println!("[CLIENT] Refreshing app list");
        // w_window_clone.set_child(Some(&w_picker_loading_box_clone));

        let main_picker_box_widget = w_main_stack_clone
            .child_by_name("picker")
            .expect("Picker stack child not found");
        let main_picker_box = main_picker_box_widget
            .downcast_ref::<gtk::Box>()
            .expect("Picker stack child is not a Box");

        main_picker_box.remove(&w_picker_scrolled_window_clone);
        main_picker_box.append(&w_picker_loading_box_clone);

        let m_picker_loading_sender = m_picker_apps_sender.clone();
        //m_picker_loading_sender.send_blocking(SteamResponse::Pending).unwrap();

        gio::spawn_blocking(move || {
            let result = GetOwnedAppList.request();
            m_picker_loading_sender.send_blocking(result).unwrap();
        });
    });

    w_header_bar_back_button.connect_clicked(move |_| {
        dev_println!("[CLIENT] Back to picker");
        w_header_bar_back_button_clone2.set_visible(false);
        w_main_stack_clone3.set_visible_child_name("picker");

        StopApps.request();
    });

    // TODO: connect on single click
    // This only connects on double click. How can I connect on click?
    w_picker_list_view.connect_activate(move |list_view, position| {
        let model = list_view.model().expect("The model has to exist.");
        let steam_app_object = model
            .item(position)
            .and_downcast::<gtk_wrappers::GSteamAppObject>()
            .expect("The item has to be an `IntegerObject`.");

        dev_println!("[CLIENT] Selected app: {}", steam_app_object.app_name());
        w_main_stack_clone2.set_visible_child_name("app");
        w_header_bar_back_button_clone.set_visible(true);

        if (LaunchApp {
            app_id: steam_app_object.app_id(),
        })
        .request()
        .is_none()
        {
            return;
        }

        // TODO: make this async
        let achievements = GetAchievements {
            app_id: steam_app_object.app_id(),
        }
        .request();

        // TODO: make this async
        let stats = GetStats {
            app_id: steam_app_object.app_id(),
        }
        .request();
    });

    glib::spawn_future_local(clone!(
        #[weak]
        w_window,
        async move {
            while let Ok(res) = m_picker_apps_receiver.recv().await {
                picker_model_clone.remove_all();
                dev_println!("[CLIENT] Callback from receiving apps");
                match res {
                    Some(apps) => {
                        let vector: Vec<gtk_wrappers::GSteamAppObject> = apps
                            .into_iter()
                            .map(gtk_wrappers::GSteamAppObject::new)
                            .collect();

                        picker_model_clone.extend_from_slice(&vector);

                        w_header_bar_refresh_button.set_sensitive(true);

                        let main_picker_box_widget = w_main_stack
                            .child_by_name("picker")
                            .expect("Picker stack child not found");
                        let main_picker_box = main_picker_box_widget
                            .downcast_ref::<gtk::Box>()
                            .expect("Picker stack child is not a Box");

                        main_picker_box.remove(&w_picker_loading_box);
                        main_picker_box.append(&w_picker_scrolled_window);

                        // w_window.set_child(Some(&w_picker_scrolled_window));
                        w_spinner.stop();
                    }
                    None => {
                        gtk::MessageDialog::new(
                            None::<&gtk::Window>,
                            gtk::DialogFlags::empty(),
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Ok,
                            &format!("Error getting applist"),
                        );
                        w_header_bar_refresh_button.set_sensitive(true);

                        let main_picker_box_widget = w_main_stack
                            .child_by_name("picker")
                            .expect("Picker stack child not found");
                        let main_picker_box = main_picker_box_widget
                            .downcast_ref::<gtk::Box>()
                            .expect("Picker stack child is not a Box");

                        main_picker_box.remove(&w_picker_loading_box);
                        main_picker_box.append(&w_picker_scrolled_window);

                        w_spinner.stop();
                    }
                }
            }
        }
    ));

    w_window.present();

    // TODO: try_connect pendant 5 secondes, et message d'erreur si échec
    // w_header_bar_refresh_button_clone.emit_clicked();
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