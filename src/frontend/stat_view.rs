use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

use gtk::gio::{spawn_blocking, ListStore};
use gtk::glib::object::Cast;
use gtk::pango::EllipsizeMode;
use gtk::prelude::{AdjustmentExt, BoxExt, GObjectPropertyExpressionExt};
use gtk::{Adjustment, Align, Box, FilterListModel, Label, ListBox, Orientation, SelectionMode, SpinButton, StringFilter, StringFilterMatchMode};

use super::request::{Request, SetIntStat};
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
    app_stat_list.set_selection_mode(SelectionMode::None);

    let app_id_clone = app_id.clone();
    app_stat_list.bind_model(Some(&app_stat_filter_model), move |item| {
        let sender = RefCell::new(channel().0);
        let stat = item.downcast_ref::<GStatObject>()
            .expect("Needs to be a GStatObject"); 

        let adjustment = Adjustment::builder()
            .lower(i32::MIN as f64)
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

        let spin_button = SpinButton::builder()
            .digits(if stat.is_integer() { 0 } else { 2 })
            .adjustment(&adjustment)
            .build();

        let app_id = app_id_clone.get().unwrap_or_default();
        let stat_id = stat.id().clone();
        let integer_stat = stat.is_integer();
        spin_button.connect_value_changed(move |button| {
            if sender.borrow_mut().send(button.value()).is_ok() { return }
            let (new_sender, receiver) = channel();
            *sender.borrow_mut() = new_sender;
            let mut value = button.value();
            let stat_id = stat_id.clone();
            //merge subsequent changes within 800ms
            spawn_blocking(move || {
                while let Ok(new) = receiver.recv_timeout(Duration::from_millis(800)) {
                    value = new;
                }
                if integer_stat {
                    SetIntStat {
                        app_id,
                        stat_id,
                        value: value as i32
                    }.request();
                } else {
                    SetIntStat {
                        app_id,
                        stat_id,
                        value: value as i32
                    }.request();
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
        stat_box.append(&button_box);
        stat_box.into()
    });

    (app_stat_list, app_stat_model, app_stat_string_filter)
}