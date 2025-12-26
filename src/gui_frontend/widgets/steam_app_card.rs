use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct SteamAppCard(ObjectSubclass<imp::SteamAppCard>)
        @extends gtk::Widget;
}

impl Default for SteamAppCard {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SteamAppCard {
    pub fn new(app: &GSteamAppObject) -> Self {
        glib::Object::builder().property("app-object", app).build()
    }
}

mod imp {
    use super::*;
    use crate::gui_frontend::widgets::gradient_overlay::GradientOverlay;
    use glib::Properties;
    use gtk::{Box, Image, Label, Orientation};
    use std::cell::RefCell;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SteamAppCard)]
    pub struct SteamAppCard {
        #[property(get, set = Self::set_app_object, explicit_notify)]
        pub app_object: RefCell<Option<GSteamAppObject>>,

        pub overlay: gtk::Overlay,
        pub gradient: GradientOverlay,
        pub main_layout: gtk::Box,
        pub image: ShimmerImage,
        pub filler_box: gtk::Box,

        // UI elements pinned to bottom
        pub bottom_container: gtk::Box,
        pub name_label: gtk::Label,
        #[property(get)]
        pub launch_button: gtk::Button,
        pub manage_button_box: gtk::Box, // Contains Manage + New
        #[property(get)]
        pub manage_button: gtk::Button,
        #[property(get)]
        pub manage_button_new: gtk::Button,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SteamAppCard {
        const NAME: &'static str = "SteamAppCardWidget";
        type Type = super::SteamAppCard;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SteamAppCard {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            // 1. Setup Base Layout
            self.filler_box.set_vexpand(true);
            self.main_layout.set_orientation(Orientation::Vertical);
            self.main_layout.append(&self.image);
            self.main_layout.append(&self.filler_box);

            // 2. Setup Launch Button
            let launch_icon = Image::builder()
                .icon_name("media-playback-start-symbolic")
                .pixel_size(11)
                .build();
            let launch_label = Label::new(Some("Launch"));
            let launch_box = Box::builder()
                .spacing(8)
                .margin_start(10)
                .margin_end(10)
                .build();
            launch_box.append(&launch_icon);
            launch_box.append(&launch_label);
            self.launch_button.add_css_class("opaque");
            self.launch_button.set_child(Some(&launch_box));

            // 3. Setup Manage Button (Linked)
            let manage_icon = Image::builder()
                .icon_name("document-edit-symbolic")
                .pixel_size(11)
                .build();
            let manage_label = Label::new(Some("Manage"));
            let attr_list = gtk::pango::AttrList::new();
            attr_list.insert(gtk::pango::AttrInt::new_weight(
                gtk::pango::Weight::Semibold,
            ));
            manage_label.set_attributes(Some(&attr_list));
            let manage_inner_box = Box::builder()
                .spacing(8)
                .margin_start(10)
                .margin_end(10)
                .build();
            manage_inner_box.append(&manage_icon);
            manage_inner_box.append(&manage_label);

            self.manage_button.set_child(Some(&manage_inner_box));
            self.manage_button.add_css_class("suggested-action");
            self.manage_button_new.add_css_class("opaque");
            self.manage_button_new.set_icon_name("window-new-symbolic");
            if let Some(img) = self
                .manage_button_new
                .child()
                .and_then(|c| c.downcast::<Image>().ok())
            {
                img.set_pixel_size(11);
            }

            self.manage_button_box
                .set_orientation(Orientation::Horizontal);
            self.manage_button_box.add_css_class("linked");
            self.manage_button_box.append(&self.manage_button);
            self.manage_button_box.append(&self.manage_button_new);

            // 4. Setup Name Label (Bold, Left Aligned)
            let color_attr = gtk::pango::AttrColor::new_foreground(65535, 65535, 65535);
            let attr_list = gtk::pango::AttrList::new();
            attr_list.insert(gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Bold));
            attr_list.insert(gtk::pango::AttrColor::new_foreground(65535, 65535, 65535));
            attr_list.insert(gtk::pango::AttrSize::new(14 * gtk::pango::SCALE));
            attr_list.insert(color_attr);
            self.name_label.set_attributes(Some(&attr_list));
            self.name_label.set_halign(gtk::Align::Start);
            self.name_label.set_margin_start(20);
            self.name_label.set_margin_end(20);
            self.name_label
                .set_ellipsize(gtk::pango::EllipsizeMode::Middle);

            // 5. Setup Bottom Overlay Container
            let button_row = Box::builder()
                .orientation(Orientation::Horizontal)
                .margin_start(20)
                .margin_bottom(20)
                .margin_top(5)
                .spacing(10)
                .height_request(33)
                .build();
            button_row.append(&self.manage_button_box);
            button_row.append(&self.launch_button);

            self.bottom_container.set_orientation(Orientation::Vertical);
            self.bottom_container.set_valign(gtk::Align::End);
            self.bottom_container.append(&self.name_label);
            self.bottom_container.append(&button_row);

            // 6. Final Assembly
            self.overlay.set_child(Some(&self.main_layout));

            self.gradient.set_hexpand(true);
            self.gradient.set_vexpand(true);
            self.overlay.add_overlay(&self.gradient);

            self.overlay.add_overlay(&self.bottom_container);

            self.overlay.set_parent(&*obj);
            obj.set_overflow(gtk::Overflow::Hidden);

            // 7. Setup Expressions (The Path B way)
            let app_obj_expr = obj.property_expression("app-object");
            app_obj_expr
                .chain_property::<GSteamAppObject>("app_name")
                .bind(&self.name_label, "label", gtk::Widget::NONE);
            app_obj_expr
                .chain_property::<GSteamAppObject>("image_url")
                .bind(&self.image, "url", gtk::Widget::NONE);
        }

        fn dispose(&self) {
            self.overlay.unparent();
        }
    }

    impl WidgetImpl for SteamAppCard {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (min, nat, b_min, b_nat) = self.overlay.measure(orientation, for_size);
            if orientation == gtk::Orientation::Vertical {
                (0, 0, b_min, b_nat)
            } else {
                (min, nat, b_min, b_nat)
            }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.overlay
                .size_allocate(&gtk::Allocation::new(0, 0, width, height), baseline);

            let (_, image_nat_h, _, _) = self.image.measure(gtk::Orientation::Vertical, width);
            let internal_h = height.max(image_nat_h);

            self.main_layout
                .size_allocate(&gtk::Allocation::new(0, 0, width, internal_h), -1);
            self.image
                .size_allocate(&gtk::Allocation::new(0, 0, width, image_nat_h), -1);
            self.filler_box.size_allocate(
                &gtk::Allocation::new(0, image_nat_h, width, (height - image_nat_h).max(0)),
                -1,
            );
        }
    }

    impl SteamAppCard {
        fn set_app_object(&self, app: Option<GSteamAppObject>) {
            if self.app_object.borrow().as_ref() == app.as_ref() {
                return;
            }

            match app {
                Some(ref _app_obj) => {}
                None => {
                    self.name_label.set_label("...");
                    self.image.set_url("");
                }
            }

            self.app_object.replace(app);
            self.obj().notify_app_object();
            self.obj().queue_resize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::app_lister::{AppModel, AppModelType};
    use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;

    #[test]
    #[ignore] // Requires a display environment
    fn test_steam_app_card_grid_layout() {
        gtk::init().expect("Failed to initialize GTK");

        // 1. Create the App List Model and Factory
        let list_store = gtk::gio::ListStore::new::<GSteamAppObject>();
        let list_factory = gtk::SignalListItemFactory::new();

        // Replicate your Factory Logic
        list_factory.connect_setup(move |_, list_item| {
            let entry = SteamAppCard::default();
            entry.set_size_request(400, 150);
            entry.set_margin_start(5);
            entry.set_margin_end(5);
            entry.set_margin_top(5);
            entry.set_margin_bottom(5);

            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be a ListItem");

            list_item.set_activatable(false);
            list_item.set_child(Some(&entry));
        });

        // 2. Setup the GridView and ScrolledWindow
        let list_selection_model = gtk::NoSelection::new(Some(list_store.clone()));

        let grid_view = gtk::GridView::builder()
            .min_columns(2)
            .margin_start(10)
            .margin_end(10)
            .model(&list_selection_model)
            .factory(&list_factory)
            .css_name("unstyled-gridview")
            .build();

        let list_scrolled_window = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_width(360)
            .child(&grid_view) // Directly child for testing
            .build();

        // 3. Replicate the HeaderBar and Window Structure
        let header_bar = gtk::HeaderBar::builder().show_title_buttons(true).build();

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Name or AppId (Ctrl+K)")
            .build();
        header_bar.pack_start(&search_entry);

        // 4. Populate with Mock Data
        for i in 0..10 {
            let app_model = AppModel {
                app_id: 440 + i,
                app_name: format!("Steam Game {}", i),
                image_url: None,
                app_type: AppModelType::App,
                developer: "Valve".to_string(),
                metacritic_score: Some(90),
            };
            list_store.append(&GSteamAppObject::new(app_model));
        }

        // 5. Final Window Assembly
        let window = gtk::ApplicationWindow::builder()
            .title("Layout Debugger")
            .default_width(850)
            .default_height(600)
            .child(&list_scrolled_window)
            .titlebar(&header_bar)
            .build();

        window.present();

        // Simple loop to keep window open
        let context = gtk::glib::MainContext::default();
        while window.is_visible() {
            context.iteration(true);
        }
    }
}
