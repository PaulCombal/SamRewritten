use glib::Object;
use gtk::glib;
use crate::backend::stat_definitions::StatInfo;

glib::wrapper! {
    pub struct GStatObject(ObjectSubclass<imp::GStatObject>);
}

impl GStatObject {
    pub fn new(info: StatInfo) -> Self {
        match info {
            StatInfo::Float(info) => Object::builder()
                .property("id", info.id)
                .property("display-name", info.display_name)
                .property("original-value", info.original_value as f64)
                .property("current-value", info.float_value as f64)
                .property("is-integer", false)
                .build(),
            StatInfo::Integer(info) => Object::builder()
                .property("id", info.id)
                .property("display-name", info.display_name)
                .property("original-value", info.original_value as f64)
                .property("current-value", info.int_value as f64)
                .property("is-integer", true)
                .build(),
        } 
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GStatObject)]
    pub struct GStatObject {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        display_name: RefCell<String>,

        #[property(get, set)]
        original_value: Cell<f64>,

        #[property(get, set)]
        current_value: Cell<f64>,

        #[property(get, set)]
        is_integer: Cell<bool>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GStatObject {
        const NAME: &'static str = "GStatObject";
        type Type = super::GStatObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GStatObject {}
}