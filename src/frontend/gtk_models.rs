use std::cell::{Cell, RefCell};
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use crate::frontend::gtk_wrappers;

#[derive(Properties, Default)]
#[properties(wrapper_type = gtk_wrappers::GSteamAppObject)]
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
    type Type = gtk_wrappers::GSteamAppObject;
}


// Trait shared by all GObjects
#[glib::derived_properties]
impl ObjectImpl for GSteamAppObject {}