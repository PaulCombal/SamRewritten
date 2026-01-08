use crate::{APP_ID, dev_println};
use gtk::gio::Settings;

pub fn get_settings() -> Settings {
    if let Ok(schema_dir) = std::env::var("APPDIR") {
        // AppImages
        let source = gtk::gio::SettingsSchemaSource::from_directory(schema_dir, None, false)
            .expect("Could not find the 'gschemas.compiled' file in the AppDir folder.");
        let schema = source
            .lookup(APP_ID, true)
            .unwrap_or_else(|| panic!("Schema '{}' not found in the schema", APP_ID));
        return Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
    }

    if let Ok(snap_name) = std::env::var("SNAP_NAME")
        && snap_name != "samrewritten"
    {
        // Dev config
        dev_println!("[CLIENT] Loading settings from dev config..");
        let schema_dir = "./assets";
        let source = gtk::gio::SettingsSchemaSource::from_directory(schema_dir, None, false)
            .expect("Could not find the 'gschemas.compiled' file in the assets folder.");
        let schema = source
            .lookup(APP_ID, true)
            .unwrap_or_else(|| panic!("Schema '{}' not found in the schema", APP_ID));
        return Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
    }

    if let Ok(schema_dir) = std::env::var("SAM_GSCHEMA_DIR_FALLBACK") {
        let source = gtk::gio::SettingsSchemaSource::from_directory(schema_dir, None, false)
            .expect("Could not find the 'gschemas.compiled' file in the fallback folder.");
        let schema = source
            .lookup(APP_ID, true)
            .unwrap_or_else(|| panic!("Schema '{}' not found in the schema", APP_ID));
        return Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
    }

    // Arch, Windows
    Settings::new(APP_ID)
}
