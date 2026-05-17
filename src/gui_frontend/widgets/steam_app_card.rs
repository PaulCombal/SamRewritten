use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::sync::atomic::AtomicBool;

pub static ANIMATIONS_DISABLED: AtomicBool = AtomicBool::new(false);

glib::wrapper! {
    pub struct SteamAppCard(ObjectSubclass<imp::SteamAppCard>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
    use gtk::glib::clone;
    use gtk::{Box, Image, Orientation};
    use std::cell::{Cell, RefCell};

    const HOVER_DURATION_MS: f64 = 200.0;
    const BUTTONS_OPACITY_REST: f64 = 0.85;
    const BUTTONS_OPACITY_HOVER: f64 = 1.0;
    const BADGE_OPACITY: f64 = 0.9;
    const BADGE_FADE_DURATION_MS: f64 = 200.0;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SteamAppCard)]
    pub struct SteamAppCard {
        #[property(get, set = Self::set_app_object, explicit_notify)]
        pub app_object: RefCell<Option<GSteamAppObject>>,
        #[property(get, set)]
        pub is_selected: std::cell::Cell<bool>,

        pub hover_progress: Cell<f64>,
        pub hover_target: Cell<f64>,
        pub hover_last_frame: Cell<i64>,
        pub hover_animating: Cell<bool>,

        pub overlay: gtk::Overlay,
        pub gradient: GradientOverlay,
        pub main_layout: gtk::Box,
        pub image: ShimmerImage,
        pub filler_box: gtk::Box,

        // UI elements over the overlay
        pub bottom_container: gtk::Box,
        pub button_row: gtk::Box,
        pub name_label: gtk::Label,
        #[property(get)]
        pub launch_button: gtk::Button,
        #[property(get)]
        pub idle_button: gtk::ToggleButton,
        pub manage_button_box: gtk::Box, // Contains Manage + New
        #[property(get)]
        pub manage_button: gtk::Button,
        #[property(get)]
        pub manage_button_new: gtk::Button,
        pub check_button: gtk::CheckButton,
        pub achievement_badge: gtk::Label,
        pub badge_loaded_handler: RefCell<Option<glib::SignalHandlerId>>,
        pub badge_fade_generation: Cell<u64>,
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
            // 200 is hardcoded because we set a height request on the card
            // Ideally, you want the value to be obj.height + 5
            self.image.set_placeholder_height(200);
            self.filler_box.set_vexpand(true);
            self.main_layout.set_orientation(Orientation::Vertical);
            self.main_layout.append(&self.image);
            self.main_layout.append(&self.filler_box);

            // 2. Setup Launch Button
            let launch_icon = Image::builder()
                .icon_name("media-playback-start-symbolic")
                .pixel_size(11)
                .build();
            let launch_label = gtk::Label::new(Some("Launch"));
            let launch_box = Box::builder()
                .spacing(8)
                .margin_start(10)
                .margin_end(10)
                .build();
            launch_box.append(&launch_icon);
            launch_box.append(&launch_label);
            self.launch_button.add_css_class("opaque");
            self.launch_button.set_child(Some(&launch_box));

            // 2b. Setup Idle Toggle Button
            let idle_icon = Image::builder()
                .icon_name("emoji-recent-symbolic")
                .pixel_size(11)
                .build();
            let idle_label = gtk::Label::new(Some("Idle"));
            let idle_box = Box::builder()
                .spacing(8)
                .margin_start(10)
                .margin_end(10)
                .build();
            idle_box.append(&idle_icon);
            idle_box.append(&idle_label);
            self.idle_button.add_css_class("opaque");
            self.idle_button.set_child(Some(&idle_box));

            // 3. Setup Manage Button (Linked)
            let manage_icon = Image::builder()
                .icon_name("document-edit-symbolic")
                .pixel_size(11)
                .build();
            let manage_label = gtk::Label::new(Some("Manage"));
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
            self.manage_button_new
                .set_tooltip_text(Some("Manage this app in a new window"));
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
            self.button_row.set_orientation(Orientation::Horizontal);
            self.button_row.set_margin_start(20);
            self.button_row.set_margin_bottom(20);
            self.button_row.set_margin_top(5);
            self.button_row.set_spacing(10);
            self.button_row.set_height_request(33);
            self.button_row.set_opacity(BUTTONS_OPACITY_REST);
            self.button_row.append(&self.manage_button_box);
            self.button_row.append(&self.idle_button);
            self.button_row.append(&self.launch_button);

            self.bottom_container.set_orientation(Orientation::Vertical);
            self.bottom_container.set_valign(gtk::Align::End);
            self.bottom_container.append(&self.name_label);
            self.bottom_container.append(&self.button_row);

            // 6. Setup Checkbox (Top Left)
            self.check_button.add_css_class("osd");
            self.check_button.set_halign(gtk::Align::Start);
            self.check_button.set_valign(gtk::Align::Start);
            self.check_button.set_margin_top(15);
            self.check_button.set_margin_start(15);

            // 6b. Setup Achievement Badge (Top Right)
            self.achievement_badge.add_css_class("osd");
            self.achievement_badge.set_halign(gtk::Align::End);
            self.achievement_badge.set_valign(gtk::Align::Start);
            self.achievement_badge.set_margin_top(15);
            self.achievement_badge.set_margin_end(15);
            self.achievement_badge.set_opacity(0.0);

            // 7. Final Assembly
            self.overlay.set_child(Some(&self.main_layout));

            self.gradient.set_hexpand(true);
            self.gradient.set_vexpand(true);
            self.overlay.add_overlay(&self.gradient);

            self.overlay.add_overlay(&self.bottom_container);
            self.overlay.add_overlay(&self.check_button);
            self.overlay.add_overlay(&self.achievement_badge);

            self.overlay.set_parent(&*obj);
            obj.set_overflow(gtk::Overflow::Hidden);

            // 8. Setup Expressions
            let app_obj_expr = obj.property_expression("app-object");
            app_obj_expr
                .chain_property::<GSteamAppObject>("app_name")
                .bind(&self.name_label, "label", gtk::Widget::NONE);
            app_obj_expr
                .chain_property::<GSteamAppObject>("image_url")
                .bind(&self.image, "url", gtk::Widget::NONE);
            app_obj_expr
                .chain_property::<GSteamAppObject>("is_idling")
                .bind(&self.idle_button, "active", gtk::Widget::NONE);

            // Idle button is sensitive when this app is currently idling
            // (so it can be stopped) or when the global idle cap allows
            // starting a new one. `can_start_idling` is kept in sync
            // across the store by the controller code.
            let idle_sensitive_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let is_idling = values
                    .get(1)
                    .and_then(|v| v.get::<bool>().ok())
                    .unwrap_or(false);
                let can_start = values
                    .get(2)
                    .and_then(|v| v.get::<bool>().ok())
                    .unwrap_or(true);
                Some((is_idling || can_start).to_value())
            });
            let idle_sensitive_expr = gtk::ClosureExpression::new::<bool>(
                &[
                    app_obj_expr.chain_property::<GSteamAppObject>("is_idling"),
                    app_obj_expr.chain_property::<GSteamAppObject>("can_start_idling"),
                ],
                idle_sensitive_closure,
            );
            idle_sensitive_expr.bind(&self.idle_button, "sensitive", gtk::Widget::NONE);

            let badge_label_closure = glib::RustClosure::new(|values: &[glib::Value]| {
                let loaded = values
                    .get(1)
                    .and_then(|v| v.get::<bool>().ok())
                    .unwrap_or(false);
                let total = values.get(2).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
                let unlocked = values.get(3).and_then(|v| v.get::<u32>().ok()).unwrap_or(0);
                let label = if loaded && total > 0 {
                    let percent = (unlocked as f64 / total as f64 * 100.0).round() as u32;
                    format!("{percent}% • {unlocked} / {total}")
                } else {
                    String::new()
                };
                Some(label.to_value())
            });
            let badge_label_expr = gtk::ClosureExpression::new::<String>(
                &[
                    app_obj_expr.chain_property::<GSteamAppObject>("achievements_loaded"),
                    app_obj_expr.chain_property::<GSteamAppObject>("achievement_count"),
                    app_obj_expr.chain_property::<GSteamAppObject>("unlocked_achievement_count"),
                ],
                badge_label_closure,
            );
            badge_label_expr.bind(&self.achievement_badge, "label", gtk::Widget::NONE);

            let opacity_closure = glib::RustClosure::new(move |values: &[glib::Value]| {
                let is_selected = values
                    .get(1)
                    .and_then(|val| val.get::<bool>().ok())
                    .unwrap_or(false);

                let opacity = if is_selected { 1.0f64 } else { 0.5f64 };
                Some(opacity.to_value())
            });

            let opacity_expr = gtk::ClosureExpression::new::<f64>(
                &[obj.property_expression("is-selected")],
                opacity_closure,
            );

            obj.bind_property("is-selected", &self.check_button, "active")
                .sync_create()
                .bidirectional()
                .build();
            opacity_expr.bind(&self.check_button, "opacity", gtk::Widget::NONE);

            // 9. Behavior
            let motion = gtk::EventControllerMotion::new();
            motion.connect_enter(clone!(
                #[weak]
                obj,
                move |_, _, _| {
                    obj.imp().start_hover_anim(1.0);
                }
            ));
            motion.connect_leave(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.imp().start_hover_anim(0.0);
                }
            ));
            obj.add_controller(motion);

            let gesture = gtk::GestureClick::new();
            gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
            let gesture = gtk::GestureClick::new();
            gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
            gesture.connect_pressed(clone!(
                #[weak]
                obj,
                move |gesture, _, x, y| {
                    // Find the widget exactly at the click coordinates
                    let target = obj.pick(x, y, gtk::PickFlags::DEFAULT);

                    if let Some(widget) = target {
                        // If the user clicked a Button or the CheckButton,
                        // do NOT claim the event. Let it bubble down to them.
                        if widget.ancestor(gtk::Button::static_type()).is_some()
                            || widget.ancestor(gtk::CheckButton::static_type()).is_some()
                        {
                            gesture.set_state(gtk::EventSequenceState::Denied);
                            return;
                        }
                    }

                    // Otherwise, it was a click on the card background.
                    // Claim it to prevent the GridView from selecting the row.
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                }
            ));
            self.overlay.add_controller(gesture);
        }

        fn dispose(&self) {
            if let Some(handler) = self.badge_loaded_handler.borrow_mut().take()
                && let Some(app) = self.app_object.borrow().as_ref()
            {
                app.disconnect(handler);
            }
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

            let overflow = (image_nat_h - height).max(0);
            let eased = self.hover_progress.get();
            let y_offset = -((overflow as f64 * eased) as i32);

            self.button_row.set_opacity(
                BUTTONS_OPACITY_REST + (BUTTONS_OPACITY_HOVER - BUTTONS_OPACITY_REST) * eased,
            );

            self.image
                .size_allocate(&gtk::Allocation::new(0, y_offset, width, image_nat_h), -1);
            self.filler_box.size_allocate(
                &gtk::Allocation::new(0, image_nat_h, width, (height - image_nat_h).max(0)),
                -1,
            );
        }
    }

    impl SteamAppCard {
        fn start_hover_anim(&self, target: f64) {
            self.hover_target.set(target);
            if ANIMATIONS_DISABLED.load(std::sync::atomic::Ordering::Relaxed) {
                self.hover_progress.set(0.0);
                self.obj().queue_allocate();
                return;
            }
            if self.hover_animating.get() {
                return;
            }
            if (self.hover_progress.get() - target).abs() < f64::EPSILON {
                return;
            }
            self.hover_animating.set(true);
            self.hover_last_frame.set(0);
            self.obj().add_tick_callback(|widget, clock| {
                let Some(card) = widget.downcast_ref::<super::SteamAppCard>() else {
                    return glib::ControlFlow::Break;
                };
                let imp = card.imp();

                let target = imp.hover_target.get();
                if ANIMATIONS_DISABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    imp.hover_progress.set(0.0);
                    imp.hover_last_frame.set(0);
                    imp.hover_animating.set(false);
                    card.queue_allocate();
                    return glib::ControlFlow::Break;
                }

                let now = clock.frame_time();
                let last = imp.hover_last_frame.get();
                let dt_ms = if last == 0 {
                    16.0
                } else {
                    (now - last) as f64 / 1000.0
                };
                imp.hover_last_frame.set(now);

                let progress = imp.hover_progress.get();
                let step = dt_ms / HOVER_DURATION_MS;
                let new_progress = if target > progress {
                    (progress + step).min(target)
                } else {
                    (progress - step).max(target)
                };
                imp.hover_progress.set(new_progress);
                card.queue_allocate();

                if (new_progress - target).abs() < f64::EPSILON {
                    imp.hover_last_frame.set(0);
                    imp.hover_animating.set(false);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }

        pub(super) fn start_badge_fade_in(&self) {
            self.badge_fade_generation
                .set(self.badge_fade_generation.get().wrapping_add(1));
            let gen_snapshot = self.badge_fade_generation.get();

            self.achievement_badge.set_opacity(0.0);

            if ANIMATIONS_DISABLED.load(std::sync::atomic::Ordering::Relaxed) {
                self.achievement_badge.set_opacity(BADGE_OPACITY);
                return;
            }

            let card_weak = self.obj().downgrade();
            let start_time = Cell::new(0i64);
            self.obj().add_tick_callback(move |_, clock| {
                let Some(card) = card_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                let imp = card.imp();
                if imp.badge_fade_generation.get() != gen_snapshot {
                    return glib::ControlFlow::Break;
                }
                let now = clock.frame_time();
                if start_time.get() == 0 {
                    start_time.set(now);
                }
                let elapsed_ms = (now - start_time.get()) as f64 / 1000.0;
                let progress = (elapsed_ms / BADGE_FADE_DURATION_MS).min(1.0);
                imp.achievement_badge.set_opacity(progress * BADGE_OPACITY);
                if progress >= 1.0 {
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }

        fn set_app_object(&self, app: Option<GSteamAppObject>) {
            if self.app_object.borrow().as_ref() == app.as_ref() {
                return;
            }

            if let Some(handler) = self.badge_loaded_handler.borrow_mut().take()
                && let Some(old) = self.app_object.borrow().as_ref()
            {
                old.disconnect(handler);
            }
            self.badge_fade_generation
                .set(self.badge_fade_generation.get().wrapping_add(1));

            match app {
                Some(ref new_app) => {
                    let loaded = new_app.achievements_loaded();
                    let total = new_app.achievement_count();
                    let target = if loaded && total > 0 {
                        BADGE_OPACITY
                    } else {
                        0.0
                    };
                    self.achievement_badge.set_opacity(target);

                    if !loaded {
                        let card_weak = self.obj().downgrade();
                        let handler = new_app.connect_achievements_loaded_notify(move |app| {
                            let Some(card) = card_weak.upgrade() else {
                                return;
                            };
                            if app.achievements_loaded() && app.achievement_count() > 0 {
                                card.imp().start_badge_fade_in();
                            }
                        });
                        *self.badge_loaded_handler.borrow_mut() = Some(handler);
                    }
                }
                None => {
                    self.name_label.set_label("...");
                    self.image.set_url("");
                    self.achievement_badge.set_opacity(0.0);
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
                playtime_minutes: None,
                last_played: None,
                achievement_count: None,
                unlocked_achievement_count: None,
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
