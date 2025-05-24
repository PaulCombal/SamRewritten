use glib::Object;
use gtk::glib;

use crate::backend::stat_definitions::AchievementInfo;

glib::wrapper! {
    pub struct GAchievementObject(ObjectSubclass<imp::GAchievementObject>);
}

impl GAchievementObject {
    pub fn new(info: AchievementInfo) -> Self {
        Object::builder()
            .property("search-text", format!("{} {}", info.name, info.description))
            .property("id", info.id)
            .property("name", info.name)
            .property("description", info.description)
            .property("is-achieved", info.is_achieved)
            .property("unlock-time", info.unlock_time.map(|time| format!("{time:#?}")))
            .property("icon-normal", info.icon_normal)
            .property("icon-locked", info.icon_locked)
            .property("permission", info.permission) 
            .build()
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GAchievementObject)]
    pub struct GAchievementObject {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set)]
        description: RefCell<String>,

        #[property(get, set)]
        search_text: RefCell<String>,

        #[property(get, set)]
        is_achieved: Cell<bool>,

        #[property(get, set)]
        unlock_time: RefCell<Option<String>>,

        #[property(get, set)]
        icon_normal: RefCell<String>,

        #[property(get, set)]
        icon_locked: RefCell<String>,

        #[property(get, set)]
        permission: RefCell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GAchievementObject {
        const NAME: &'static str = "GAchievementObject";
        type Type = super::GAchievementObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GAchievementObject {}
}

