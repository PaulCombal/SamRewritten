use crate::{APP_ID, dev_println};
use gtk::gio::Settings;
use std::sync::OnceLock;

struct SendSyncSettings(Settings);
unsafe impl Send for SendSyncSettings {}
unsafe impl Sync for SendSyncSettings {}

pub fn get_settings() -> &'static Settings {
    static SETTINGS: OnceLock<SendSyncSettings> = OnceLock::new();

    let wrapper = SETTINGS.get_or_init(|| {
        let settings = if let Ok(schema_dir) = std::env::var("APPDIR") {
            let source = gtk::gio::SettingsSchemaSource::from_directory(&schema_dir, None, false)
                .expect("Could not find gschemas.compiled in APPDIR");
            let schema = source.lookup(APP_ID, true).expect("Schema not found");
            Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None)
        } else if let Ok(snap_name) = std::env::var("SNAP_NAME")
            && snap_name != "samrewritten"
        {
            dev_println!("[CLIENT] Loading settings from dev config..");
            let source = gtk::gio::SettingsSchemaSource::from_directory("./assets", None, false)
                .expect("Could not find gschemas.compiled in assets");
            let schema = source.lookup(APP_ID, true).expect("Schema not found");
            Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None)
        } else if let Ok(schema_dir) = std::env::var("SAM_GSCHEMA_DIR_FALLBACK") {
            let source = gtk::gio::SettingsSchemaSource::from_directory(&schema_dir, None, false)
                .expect("Could not find gschemas.compiled in fallback");
            let schema = source.lookup(APP_ID, true).expect("Schema not found");
            Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None)
        } else {
            Settings::new(APP_ID)
        };

        SendSyncSettings(settings)
    });

    &wrapper.0
}
