use glib::Object;
use gtk::glib;
use crate::backend::app_lister::AppModel;
use crate::frontend::gtk_models;

// ANCHOR: integer_object
glib::wrapper! {
    pub struct GSteamAppObject(ObjectSubclass<gtk_models::GSteamAppObject>);
}

impl GSteamAppObject {
    pub fn new(app: AppModel) -> Self {
        Object::builder()
            .property("app_id", app.app_id)
            .property("app_name", app.app_name.clone())
            .property("image_url", app.image_url)
            .build()
    }
}
// ANCHOR_END: integer_object