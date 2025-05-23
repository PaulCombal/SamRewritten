use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

use gtk::gio::{spawn_blocking, ListStore};
use gtk::glib::object::Cast;
use gtk::glib::clone;
use gtk::pango::EllipsizeMode;
use gtk::prelude::{AdjustmentExt, BoxExt, GObjectPropertyExpressionExt, WidgetExt};
use gtk::{glib, Adjustment, Align, Box, FilterListModel, Label, ListBox, Orientation, SelectionMode, SpinButton, StringFilter, StringFilterMatchMode};

use super::request::{Request, SetFloatStat, SetIntStat};
use super::stat::GStatObject;

pub fn create_stats_view(app_id: Rc<Cell<Option<u32>>>) -> (ListBox, ListStore, StringFilter) {
    let app_stat_model = ListStore::new::<GStatObject>();
    let app_stat_string_filter = StringFilter::builder()
        .expression(&GStatObject::this_expression("display-name"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let app_stat_filter_model = FilterListModel::builder()
        .model(&app_stat_model)
        .filter(&app_stat_string_filter)
        .build();
    let app_stat_list = ListBox::builder()
        .show_separators(true)
        .build();
    let app_id_clone = app_id.clone();
    
    app_stat_list.set_selection_mode(SelectionMode::None);
    app_stat_list.bind_model(Some(&app_stat_filter_model), move |item| {
        let sender = RefCell::new(channel().0);
        let stat = item.downcast_ref::<GStatObject>()
            .expect("Needs to be a GStatObject"); 

        let adjustment = Adjustment::builder()
            .lower(0f64)
            .upper(i32::MAX as f64)
            .page_size(0.0)
            .build();

        if stat.is_integer() {
            adjustment.set_step_increment(1.0);
            adjustment.set_value(stat.current_value());
        } else {
            adjustment.set_step_increment(0.01);
            adjustment.set_value(stat.current_value());
        }
        
        if stat.is_increment_only() {
            adjustment.set_lower(stat.current_value());
        }

        let spin_button = SpinButton::builder()
            .digits(if stat.is_integer() { 0 } else { 2 })
            .adjustment(&adjustment)
            .build();

        let app_id = app_id_clone.get().unwrap_or_default();
        let stat_id = stat.id().clone();
        let integer_stat = stat.is_integer();
        let increment_only_stat = stat.is_increment_only();
        let stat_original_value = stat.original_value();
        
        let (tx_ui_update, rx_ui_update) = channel::<(bool, f64)>();
        let adjustment_clone_for_ui = adjustment.clone();
        let spin_button_clone = spin_button.clone();

        // TODO: This feels highly unoptimized: a timeout task is ran every 30ms for each stat entry
        // All of this to update a few controls. This stems from the fact that I did not manage
        // to move the spinbutton or adjustment accross threads.
        glib::timeout_add_local(
            Duration::from_millis(30),
            clone!(#[strong] adjustment_clone_for_ui, move || {
                // println!("Running timeout task");
                while let Ok((success, value)) = rx_ui_update.try_recv() {
                    if success {
                        adjustment_clone_for_ui.set_lower(value);
                    }
                    else {
                        spin_button_clone.set_value(value);
                    }
                }
                glib::ControlFlow::Continue
            }),
        );


        spin_button.connect_value_changed(move |button| {
            if sender.borrow_mut().send(button.value()).is_ok() { return }
            let (new_sender, receiver) = channel();
            *sender.borrow_mut() = new_sender;
            let mut value = button.value();
            let stat_id = stat_id.clone();

            // Clone the sender for the UI update channel to move into the spawn_blocking closure
            let tx_ui_update_clone = tx_ui_update.clone();

            spawn_blocking(move || {
                while let Ok(new) = receiver.recv_timeout(Duration::from_millis(500)) {
                    value = new;
                }
                let res = if integer_stat {
                    SetIntStat {
                        app_id,
                        stat_id,
                        value: value as i32
                    }.request()
                } else {
                    SetFloatStat {
                        app_id,
                        stat_id,
                        value: value as f32
                    }.request()
                };

                match res {
                    Some(success) if success => {
                        if increment_only_stat {
                            if tx_ui_update_clone.send((true, value)).is_err() {
                                eprintln!("Failed to send value to main thread for UI update. Channel might be closed.");
                            }
                        }
                    },
                    _ => {
                        if tx_ui_update_clone.send((false, stat_original_value)).is_err() {
                            eprintln!("Failed to send value to main thread for UI update. Channel might be closed.");
                        }
                    }
                }
            });
        });

        let button_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::End)
            .build();
        button_box.append(&spin_button);
        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let name_label = Label::builder()
            .label(stat.display_name())
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        
        let stat_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        stat_box.append(&name_label);
        stat_box.append(&spacer);
        
        if stat.is_increment_only() {
            let icon_increment_only = gtk::Image::from_icon_name("go-up-symbolic");
            icon_increment_only.set_margin_end(8);
            icon_increment_only.set_tooltip_text(Some("Increment only"));
            stat_box.append(&icon_increment_only);
        }
        
        stat_box.append(&button_box);
        stat_box.into()
    });

    (app_stat_list, app_stat_model, app_stat_string_filter)
}