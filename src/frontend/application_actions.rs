use gtk::gio::SimpleAction;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{AboutDialog, Application};
use gtk::glib;

pub fn setup_app_actions(
    application: &Application,
    about_dialog: &AboutDialog,
    refresh_app_list_action: &SimpleAction,
    refresh_achievements_list_action: &SimpleAction,
) {
    let action_show_about_dialog = SimpleAction::new("about", None);
    action_show_about_dialog.connect_activate(clone!(#[weak] about_dialog, move |_,_| {
         about_dialog.show();
    }));

    let action_quit = SimpleAction::new("quit", None);
    action_quit.connect_activate(clone!(#[weak] application, move |_,_| {
         application.quit();
    }));

    application.add_action(refresh_app_list_action);
    application.add_action(refresh_achievements_list_action);
    application.add_action(&action_show_about_dialog);
    application.add_action(&action_quit);
}