
use gtk::gdk_pixbuf::Pixbuf;
use gtk::{AboutDialog, ApplicationWindow, Image, License, MenuButton, PopoverMenu, PositionType};
use std::io::Cursor;
use gtk::gdk::Paintable;

pub fn create_about_dialog(window: &ApplicationWindow, logo: &Paintable) -> AboutDialog {
    AboutDialog::builder()
        .modal(true)
        .transient_for(window)
        .hide_on_close(true)
        .license_type(License::Gpl30)
        .version(env!("CARGO_PKG_VERSION"))
        .program_name("SamRewritten 2")
        .authors(env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<_>>())
        .comments(env!("CARGO_PKG_DESCRIPTION"))
        .logo(logo)
        .build()
}

pub fn load_logo() -> Paintable {
    // TODO: See if the forward slash syntax works on both?
    #[cfg(target_os = "windows")]
    let image_bytes = include_bytes!("..\\..\\assets\\icon_256.png");
    #[cfg(target_os = "linux")]
    let image_bytes = include_bytes!("../../assets/icon_256.png");

    let logo_pixbuf = Pixbuf::from_read(Cursor::new(image_bytes)).expect("Failed to load logo");
    Image::from_pixbuf(Some(&logo_pixbuf)).paintable().expect("Failed to create logo image")
}

pub fn create_context_menu_button() -> (MenuButton, PopoverMenu) {
    let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").build();

    let context_menu_model = gtk::gio::Menu::new();

    // Let's remember we can add sections, but for now I don't see the use case
    // let section = gio::Menu::new();
    // section.append(Some("Sub Item A"), Some("app.subitemA"));
    // menu.append_section(Some("Section"), &section);
    context_menu_model.append(Some("Refresh app list"), Some("app.refresh_app_list"));
    context_menu_model.append(Some("Refresh achievements list"), Some("app.refresh_achievements_list"));
    context_menu_model.append(Some("About"), Some("app.about"));
    context_menu_model.append(Some("Quit"), Some("app.quit"));

    let popover = PopoverMenu::builder()
        .position(PositionType::Bottom)
        .has_arrow(true)
        .menu_model(&context_menu_model)
        .build();

    menu_button.set_popover(Some(&popover));

    (menu_button, popover)
}