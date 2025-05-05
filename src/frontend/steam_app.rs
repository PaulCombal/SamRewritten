use crate::backend::app_lister::AppModel;
use glib::Object;
use gtk::glib;

// ANCHOR: integer_object
glib::wrapper! {
    pub struct GSteamAppObject(ObjectSubclass<imp::GSteamAppObject>);
}

impl GSteamAppObject {
    pub fn new(app: AppModel) -> Self {
        Object::builder()
            .property("app_id", app.app_id)
            .property("app_name", app.app_name)
            .property("image_url", app.image_url)
            .build()
    }
}
// ANCHOR_END: integer_object

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GSteamAppObject)]
    pub struct GSteamAppObject {
        #[property(get, set)]
        app_id: Cell<u32>,

        #[property(get, set)]
        app_name: RefCell<String>,

        #[property(get, set)]
        image_url: RefCell<Option<String>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for GSteamAppObject {
        const NAME: &'static str = "GSteamAppObject";
        type Type = super::GSteamAppObject;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for GSteamAppObject {}
}
