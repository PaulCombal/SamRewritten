// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::gui_frontend::i18n::tr;
use gtk::ApplicationWindow;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{Align, Orientation};

#[cfg(all(unix, not(feature = "adwaita")))]
use std::cell::Cell;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::rc::Rc;

/// Scrollable, selectable, copyable list dialog. Use when the body may contain
/// more than ~10 entries — the plain `AlertDialog` detail string can't scroll.
/// `intro` is a static header above the scroll area; pass `""` to omit.
pub fn show_list_dialog(
    parent: &impl gtk::glib::object::IsA<gtk::Window>,
    title: &str,
    intro: &str,
    body: &str,
) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .destroy_with_parent(true)
        .default_width(560)
        .default_height(420)
        .build();

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(12)
        .build();

    if !intro.is_empty() {
        let intro_label = gtk::Label::builder()
            .label(intro)
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build();
        content.append(&intro_label);
    }

    let text_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    text_view.buffer().set_text(body);

    let scroller = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_height(200)
        .propagate_natural_height(true)
        .has_frame(true)
        .child(&text_view)
        .build();
    content.append(&scroller);

    let ok_button = gtk::Button::with_label(tr("OK").as_str());
    ok_button.add_css_class("suggested-action");
    ok_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .build();
    button_box.append(&ok_button);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

#[cfg(feature = "adwaita")]
pub fn show_message_dialog(parent: Option<&gtk::Window>, title: &str, body: &str) {
    use adw::prelude::*;

    let dialog = adw::AlertDialog::new(Some(title), Some(body));
    dialog.add_response("ok", tr("OK").as_str());
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("ok");
    dialog.present(parent);
}

#[cfg(not(feature = "adwaita"))]
pub fn show_message_dialog(parent: Option<&gtk::Window>, title: &str, body: &str) {
    let dialog = gtk::Window::builder()
        .modal(true)
        .title(title)
        .resizable(false)
        .destroy_with_parent(true)
        .default_width(380)
        .build();
    dialog.set_transient_for(parent);

    let label = gtk::Label::builder()
        .label(body)
        .wrap(true)
        .xalign(0.0)
        .build();

    let ok_button = gtk::Button::with_label(tr("OK").as_str());
    ok_button.add_css_class("suggested-action");
    ok_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .margin_top(12)
        .build();
    button_box.append(&ok_button);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(8)
        .build();
    content.append(&label);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

#[cfg(unix)]
fn show_markup_warning(parent: &ApplicationWindow, title: &str, markup: &str) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .destroy_with_parent(true)
        .default_width(520)
        .build();

    let label = gtk::Label::builder()
        .use_markup(true)
        .label(markup)
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .build();

    let close_button = gtk::Button::with_label(tr("OK").as_str());
    close_button.add_css_class("suggested-action");
    close_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .margin_top(12)
        .build();
    button_box.append(&close_button);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(8)
        .build();
    content.append(&label);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

/// Pick the Steam install once, before the main window, then call `on_chosen`
/// (`None` = locator default). Runs on every dismissal path so the caller always
/// gets to start up.
#[cfg(unix)]
pub fn choose_steam_install_then<F>(parent: &ApplicationWindow, on_chosen: F)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use crate::utils::steam_locator::SteamLocator;

    // Snap: no personal-files access; the user grants Steam via the portal.
    if crate::utils::snap::is_snap() {
        choose_steam_install_snap(parent, on_chosen);
        return;
    }

    let dirs = SteamLocator::get_local_steam_install_root_folders();

    if dirs.is_empty() {
        let full_message = format!(
            "{}\n\n{}\n\n{}\n{}",
            tr("<b>No Steam installations were found on your system.</b>"),
            tr(
                "SamRewritten couldn't find Steam in any of the standard locations. If you haven't installed Steam yet, please install it through your distribution's official repository or app store."
            ),
            tr("<b>Already have Steam installed?</b>"),
            tr(
                "If you've installed Steam in a custom location, you can point SamRewritten to it using environment variables. Please check the <a href=\"https://github.com/PaulCombal/SamRewritten\">GitHub page</a> for instructions, or to report your issue."
            ),
        );
        show_markup_warning(
            parent,
            tr("No compatible version of Steam found").as_str(),
            full_message.as_str(),
        );
        on_chosen(None);
        return;
    }

    if dirs.len() == 1 {
        on_chosen(None);
        return;
    }

    show_steam_install_chooser(parent, dirs, on_chosen);
}

/// Running-vs-not radio list shared by both chooser variants; preselects the
/// install Steam is currently running from (the only one that works).
#[cfg(unix)]
fn build_install_radio(dirs: &[PathBuf]) -> (gtk::Box, Vec<gtk::CheckButton>) {
    let radio_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    let running = crate::utils::steam_ns::running_steam_install_roots();
    let is_running = |dir: &std::path::Path| {
        std::fs::canonicalize(dir)
            .map(|c| running.contains(&c))
            .unwrap_or(false)
    };
    let default_idx = dirs.iter().position(|d| is_running(d)).unwrap_or(0);

    let buttons: Vec<gtk::CheckButton> = dirs
        .iter()
        .map(|dir| {
            let suffix = if is_running(dir) {
                tr("    (Steam is running here)")
            } else {
                tr("    (Steam not running here)")
            };
            let cb = gtk::CheckButton::with_label(&format!("{}{suffix}", dir.display()));
            radio_box.append(&cb);
            cb
        })
        .collect();
    for cb in buttons.iter().skip(1) {
        cb.set_group(Some(&buttons[0]));
    }
    buttons[default_idx].set_active(true);

    (radio_box, buttons)
}

/// Multi-install chooser (non-snap): confirm the selected install or quit.
#[cfg(unix)]
fn show_steam_install_chooser<F>(parent: &ApplicationWindow, dirs: Vec<PathBuf>, on_chosen: F)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    let (radio_box, buttons) = build_install_radio(&dirs);
    let body = tr(
        "SamRewritten found more than one Steam installation. The one Steam is currently running from is preselected — the others won't work unless you start Steam from them first.",
    );
    let hint = gtk::Label::builder()
        .use_markup(true)
        .label(
            tr("You'll be asked again next launch. To skip this for good, set the <tt>SAM_STEAM_INSTALL_ROOT</tt> environment variable to the install you want — see the <a href=\"https://github.com/PaulCombal/SamRewritten?tab=readme-ov-file#environment-variables\">README</a>.")
            .as_str(),
        )
        .wrap(true)
        .xalign(0.0)
        .build();

    let dirs = Rc::new(dirs);
    let buttons = Rc::new(buttons);
    let on_chosen = Rc::new(on_chosen);

    #[cfg(feature = "adwaita")]
    {
        use adw::prelude::*;

        let dialog = adw::AlertDialog::new(
            Some(tr("Choose a Steam installation").as_str()),
            Some(body.as_str()),
        );

        let extra = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .build();
        extra.append(&radio_box);
        extra.append(&hint);
        dialog.set_extra_child(Some(&extra));

        dialog.add_response("cancel", tr("Quit").as_str());
        dialog.add_response("use", tr("Use this installation").as_str());
        dialog.set_response_appearance("use", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("use"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak]
                parent,
                #[strong]
                dirs,
                #[strong]
                buttons,
                #[strong]
                on_chosen,
                move |_, response| {
                    if response == "use" {
                        let idx = buttons
                            .iter()
                            .position(gtk::CheckButton::is_active)
                            .unwrap_or(0);
                        on_chosen(Some(dirs[idx].clone()));
                    } else if let Some(app) = parent.application() {
                        app.quit();
                    }
                }
            ),
        );
        dialog.present(Some(parent));
    }

    #[cfg(not(feature = "adwaita"))]
    {
        let dialog = gtk::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title(tr("Choose a Steam installation").as_str())
            .destroy_with_parent(true)
            .default_width(560)
            .build();

        let intro = gtk::Label::builder()
            .label(body.as_str())
            .wrap(true)
            .xalign(0.0)
            .build();

        // Confirm selects; any other close cancels and quits (no main window yet).
        let confirmed = Rc::new(Cell::new(false));
        dialog.connect_close_request(clone!(
            #[weak]
            parent,
            #[strong]
            buttons,
            #[strong]
            dirs,
            #[strong]
            on_chosen,
            #[strong]
            confirmed,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                if confirmed.get() {
                    let idx = buttons
                        .iter()
                        .position(gtk::CheckButton::is_active)
                        .unwrap_or(0);
                    on_chosen(Some(dirs[idx].clone()));
                } else if let Some(app) = parent.application() {
                    app.quit();
                }
                glib::Propagation::Proceed
            }
        ));

        let use_button = gtk::Button::with_label(tr("Use this installation").as_str());
        use_button.add_css_class("suggested-action");
        use_button.connect_clicked(clone!(
            #[weak]
            dialog,
            #[strong]
            confirmed,
            move |_| {
                confirmed.set(true);
                dialog.close();
            }
        ));

        let button_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .spacing(8)
            .margin_top(12)
            .build();
        button_box.append(&use_button);

        let content = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .spacing(12)
            .build();
        content.append(&intro);
        content.append(&radio_box);
        content.append(&hint);
        content.append(&button_box);

        dialog.set_child(Some(&content));
        dialog.present();
    }
}

/// First run shows the explainer + picker; later runs reuse the saved grant.
#[cfg(unix)]
fn choose_steam_install_snap<F>(parent: &ApplicationWindow, on_chosen: F)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use crate::utils::snap;

    let on_chosen = Rc::new(on_chosen);

    if let Some(root) = snap::load_saved_root() {
        match snap::mirror_steamclient(&root) {
            Ok(copy) => {
                eprintln!(
                    "[SNAP] Reusing saved Steam root {} (mirrored .so -> {})",
                    root.display(),
                    copy.display()
                );
                snap::pin_install_root(&root);
                on_chosen(Some(root));
                return;
            }
            Err(e) => eprintln!("[SNAP] Saved root no longer usable ({e}); asking again"),
        }
    }

    show_snap_first_run_dialog(parent, on_chosen);
}

/// First-run explainer for the snap; two CTAs grant access, anything else quits.
#[cfg(unix)]
fn show_snap_first_run_dialog<F>(parent: &ApplicationWindow, on_chosen: Rc<F>)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use crate::utils::snap;

    let body = tr(
        "This is the sandboxed <b>Snap</b> version of SamRewritten, so it needs your permission to read your Steam files. Pick your Steam folder once — a system dialog will open, just confirm it to grant access. You won't be asked again.\n\n<b>Flatpak Steam is not supported</b> by the Snap. If you installed Steam with Flatpak, please use the AppImage instead.",
    );

    #[cfg(feature = "adwaita")]
    {
        use adw::prelude::*;

        let dialog = adw::AlertDialog::new(Some(tr("Connect to Steam").as_str()), None);
        dialog.set_body_use_markup(true);
        dialog.set_body(body.as_str());
        dialog.add_response("cancel", tr("Quit").as_str());
        dialog.add_response("browse", tr("Choose a folder…").as_str());
        dialog.add_response("snap", tr("Use Snap Steam").as_str());
        dialog.set_response_appearance("snap", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("snap"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak]
                parent,
                #[strong]
                on_chosen,
                move |_, response| match response {
                    "snap" => run_snap_portal_picker(
                        &parent,
                        on_chosen.clone(),
                        snap::snap_steam_default_path(),
                    ),
                    "browse" =>
                        run_snap_portal_picker(&parent, on_chosen.clone(), snap::real_home()),
                    _ =>
                        if let Some(app) = parent.application() {
                            app.quit();
                        },
                }
            ),
        );
        dialog.present(Some(parent));
    }

    #[cfg(not(feature = "adwaita"))]
    {
        let dialog = gtk::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title(tr("Connect to Steam").as_str())
            .destroy_with_parent(true)
            .default_width(520)
            .build();

        let intro = gtk::Label::builder()
            .use_markup(true)
            .label(body.as_str())
            .wrap(true)
            .xalign(0.0)
            .build();

        // No CTA chosen (closed/Esc) → quit; the main window is never shown.
        let proceeding = Rc::new(Cell::new(false));
        dialog.connect_close_request(clone!(
            #[weak]
            parent,
            #[strong]
            proceeding,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                if !proceeding.get() {
                    if let Some(app) = parent.application() {
                        app.quit();
                    }
                }
                glib::Propagation::Proceed
            }
        ));

        let browse_button = gtk::Button::with_label(tr("Choose a folder…").as_str());
        browse_button.connect_clicked(clone!(
            #[weak]
            dialog,
            #[weak]
            parent,
            #[strong]
            on_chosen,
            #[strong]
            proceeding,
            move |_| {
                proceeding.set(true);
                dialog.close();
                run_snap_portal_picker(&parent, on_chosen.clone(), snap::real_home());
            }
        ));

        let snap_button = gtk::Button::with_label(tr("Use Snap Steam").as_str());
        snap_button.add_css_class("suggested-action");
        snap_button.connect_clicked(clone!(
            #[weak]
            dialog,
            #[weak]
            parent,
            #[strong]
            on_chosen,
            #[strong]
            proceeding,
            move |_| {
                proceeding.set(true);
                dialog.close();
                run_snap_portal_picker(&parent, on_chosen.clone(), snap::snap_steam_default_path());
            }
        ));

        let button_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .spacing(8)
            .margin_top(12)
            .build();
        button_box.append(&browse_button);
        button_box.append(&snap_button);

        let content = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .spacing(12)
            .build();
        content.append(&intro);
        content.append(&button_box);

        dialog.set_child(Some(&content));
        dialog.present();
    }
}

/// Folder-not-usable warning that loops back to the picker on dismissal. Never
/// proceed on a bad pick: a stale steamclient.so mirrored from an earlier pick
/// talks to running Steam and would make the wrong folder look like it worked.
#[cfg(unix)]
fn show_snap_folder_error<F>(parent: &ApplicationWindow, on_chosen: Rc<F>)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(tr("Steam folder not usable").as_str())
        .destroy_with_parent(true)
        .default_width(520)
        .build();

    let label = gtk::Label::builder()
        .use_markup(true)
        .label(
            tr("SamRewritten couldn't read <tt>steamclient.so</tt> in the folder you selected. Pick the <b>Steam</b> directory — the one containing a <tt>linux64</tt> folder. A Flatpak Steam can't be used by the Snap; please use the AppImage.")
                .as_str(),
        )
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .build();

    let ok_button = gtk::Button::with_label(tr("Choose again").as_str());
    ok_button.add_css_class("suggested-action");
    ok_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    dialog.connect_close_request(clone!(
        #[weak]
        parent,
        #[strong]
        on_chosen,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_| {
            show_snap_first_run_dialog(&parent, on_chosen.clone());
            glib::Propagation::Proceed
        }
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .margin_top(12)
        .build();
    button_box.append(&ok_button);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(8)
        .build();
    content.append(&label);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

/// Usable folder → start the app; unusable → re-prompt; none (portal failure) → quit.
#[cfg(unix)]
fn finish_snap_pick<F>(parent: &ApplicationWindow, on_chosen: &Rc<F>, root: Option<PathBuf>)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use crate::utils::snap;

    let Some(root) = root else {
        eprintln!("[SNAP] Portal returned no folder; quitting");
        if let Some(app) = parent.application() {
            app.quit();
        }
        return;
    };
    eprintln!("[SNAP] Portal returned Steam root: {}", root.display());

    match snap::mirror_steamclient(&root) {
        Ok(copy) => {
            eprintln!("[SNAP] Mirrored steamclient.so -> {}", copy.display());
            snap::save_root(&root);
            snap::pin_install_root(&root);
            on_chosen(Some(root));
        }
        Err(e) => {
            eprintln!(
                "[SNAP] Could not read steamclient.so under {}: {e}",
                root.display()
            );
            show_snap_folder_error(parent, on_chosen.clone());
        }
    }
}

/// Open the XDG FileChooser portal directly over D-Bus, pre-aimed at `initial`.
/// `GtkFileDialog::set_initial_folder` can't be used: it reads the folder
/// in-process, which snap confinement denies for `~/snap/*` (spurious "unable to
/// read contents" box). The portal runs unconfined, so `current_folder` aims
/// there cleanly; hand-rolled to avoid an extra crate.
#[cfg(unix)]
fn run_snap_portal_picker<F>(parent: &ApplicationWindow, on_chosen: Rc<F>, initial: Option<PathBuf>)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use gtk::gio;
    use gtk::glib::Variant;
    use gtk::glib::variant::ToVariant;
    use std::cell::RefCell;
    use std::os::unix::ffi::OsStrExt;

    let conn = match gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SNAP] No session bus for portal: {e}; falling back");
            finish_snap_pick(parent, &on_chosen, None);
            return;
        }
    };

    // Fixed handle_token → predictable Response path; subscribe before calling.
    let token = "samrw_steam_folder";
    let sender = conn
        .unique_name()
        .map(|n| n.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let request_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    let sub_cell: Rc<RefCell<Option<gio::SignalSubscription>>> = Rc::new(RefCell::new(None));
    let sub = conn.subscribe_to_signal(
        Some("org.freedesktop.portal.Desktop"),
        Some("org.freedesktop.portal.Request"),
        Some("Response"),
        Some(&request_path),
        None,
        gio::DBusSignalFlags::NONE,
        clone!(
            #[weak]
            parent,
            #[strong]
            on_chosen,
            #[strong]
            sub_cell,
            move |sig| {
                let _unsub = sub_cell.borrow_mut().take(); // drop = unsubscribe
                let response = sig.parameters.child_value(0).get::<u32>().unwrap_or(1);
                if response != 0 {
                    eprintln!("[SNAP] Steam folder selection cancelled");
                    if let Some(app) = parent.application() {
                        app.quit();
                    }
                    return;
                }
                let results = gtk::glib::VariantDict::new(Some(&sig.parameters.child_value(1)));
                let root = results
                    .lookup_value("uris", None)
                    .and_then(|v| v.get::<Vec<String>>())
                    .and_then(|uris| uris.first().and_then(|u| gio::File::for_uri(u).path()));
                finish_snap_pick(&parent, &on_chosen, root);
            }
        ),
    );
    *sub_cell.borrow_mut() = Some(sub);

    let opts = gtk::glib::VariantDict::new(None);
    opts.insert_value("handle_token", &token.to_variant());
    opts.insert_value("directory", &true.to_variant());
    opts.insert_value("modal", &true.to_variant());
    if let Some(dir) = initial.as_ref() {
        let mut bytes = dir.as_os_str().as_bytes().to_vec();
        bytes.push(0); // current_folder is a NUL-terminated byte string
        opts.insert_value("current_folder", &Variant::array_from_fixed_array(&bytes));
    }

    let params = Variant::tuple_from_iter([
        "".to_variant(),
        tr("Select your Steam folder").to_variant(),
        opts.end(),
    ]);

    conn.call(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser",
        "OpenFile",
        Some(&params),
        Some(gtk::glib::VariantTy::new("(o)").unwrap()),
        gio::DBusCallFlags::NONE,
        -1,
        gio::Cancellable::NONE,
        clone!(
            #[weak]
            parent,
            #[strong]
            on_chosen,
            #[strong]
            sub_cell,
            move |res| {
                if let Err(e) = res {
                    eprintln!("[SNAP] portal OpenFile failed: {e}");
                    let _unsub = sub_cell.borrow_mut().take();
                    finish_snap_pick(&parent, &on_chosen, None);
                }
            }
        ),
    );
}

#[cfg(windows)]
pub fn choose_steam_install_then<F>(_parent: &ApplicationWindow, on_chosen: F)
where
    F: Fn(Option<std::path::PathBuf>) + 'static,
{
    on_chosen(None);
}
