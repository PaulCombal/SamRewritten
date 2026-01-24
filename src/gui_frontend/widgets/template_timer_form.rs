use gtk::glib;

glib::wrapper! {
    pub struct SamTimerConfigForm(ObjectSubclass<imp::SamTimerConfigForm>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SamTimerConfigForm {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SamTimerConfigForm {
    pub fn new() -> Self {
        Self::default()
    }
}

mod imp {
    use crate::gui_frontend::gsettings::get_settings;
    use crate::gui_frontend::widgets::template_achievements::SamAchievementsPage;
    use gtk::CompositeTemplate;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievement_timer_form.ui")]
    pub struct SamTimerConfigForm {
        // #[template_child]
        // pub sort_unlock_radio: TemplateChild<gtk::CheckButton>,
        // #[template_child]
        // pub sort_az_radio: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub count_input: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub percent_input: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub selected_count_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SamTimerConfigForm {
        const NAME: &'static str = "SamTimerConfigForm";
        type Type = super::SamTimerConfigForm;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SamTimerConfigForm {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let settings = get_settings();
            let group = gtk::gio::SimpleActionGroup::new();

            let initial_count = settings.uint("timed-autoselect-count");
            let initial_percent = settings.uint("timed-autoselect-percent");

            self.count_input.set_value(initial_count as f64);
            self.percent_input.set_value(initial_percent as f64);

            self.count_input.connect_value_changed(glib::clone!(
                #[weak]
                settings,
                move |spin| {
                    let val = spin.value() as u32;
                    let _ = settings.set_uint("timed-autoselect-count", val);
                    crate::dev_println!("[CLIENT] Timed count setting saved: {}", val);
                }
            ));

            self.percent_input.connect_value_changed(glib::clone!(
                #[weak]
                settings,
                move |spin| {
                    let val = spin.value() as u32;
                    let _ = settings.set_uint("timed-autoselect-percent", val);
                    crate::dev_println!("[CLIENT] Timed percent setting saved: {:.2}", val);
                }
            ));

            let initial_sort = settings.string("timed-sort-method");
            let sort_action = gtk::gio::SimpleAction::new_stateful(
                "sort-method",
                Some(&String::static_variant_type()),
                &initial_sort.to_variant(),
            );

            sort_action.connect_activate(gtk::glib::clone!(
                #[weak]
                settings,
                move |action, parameter| {
                    if let Some(target) = parameter {
                        let val: String = target.get().unwrap();
                        action.set_state(target);
                        settings.set_string("timed-sort-method", &val).unwrap();
                        crate::dev_println!("[CLIENT] Sort method changed to: {}", val);
                    }
                }
            ));

            let initial_auto = settings.string("timed-autoselect-method");
            let autoselect_action = gtk::gio::SimpleAction::new_stateful(
                "autoselect-method",
                Some(&String::static_variant_type()),
                &initial_auto.to_variant(),
            );

            autoselect_action.connect_activate(gtk::glib::clone!(
                #[weak]
                settings,
                move |action, parameter| {
                    if let Some(target) = parameter {
                        let val: String = target.get().unwrap();
                        action.set_state(target);
                        let _ = settings.set_string("timed-autoselect-method", &val);
                        crate::dev_println!("[CLIENT] Autoselect method changed to: {}", val);
                    }
                }
            ));

            let trigger_sort_action = gtk::gio::SimpleAction::new("trigger-sort", None);

            trigger_sort_action.connect_activate(glib::clone!(
                #[weak]
                obj,
                move |_, _| {
                    if let Some(page) = obj.ancestor(SamAchievementsPage::static_type()) {
                        let page = page.downcast::<SamAchievementsPage>().unwrap();
                        page.sort_store_manually();
                    }
                }
            ));

            let autoselect_trigger = gtk::gio::SimpleAction::new("trigger-autoselect", None);

            autoselect_trigger.connect_activate(glib::clone!(
                #[weak]
                obj,
                move |_, _| {
                    let settings = get_settings();
                    let method = settings.string("timed-autoselect-method");

                    if let Some(page) = obj.ancestor(SamAchievementsPage::static_type()) {
                        let page = page.downcast::<SamAchievementsPage>().unwrap();

                        if method == "count" {
                            let count = obj.imp().count_input.value() as u32;
                            page.apply_auto_selection_count(count);
                        } else if method == "percent" {
                            let percent = obj.imp().percent_input.value() as u32;
                            page.apply_auto_selection_percent(percent);
                        }
                    }
                }
            ));

            group.add_action(&autoselect_trigger);
            group.add_action(&trigger_sort_action);
            group.add_action(&sort_action);
            group.add_action(&autoselect_action);
            obj.insert_action_group("config", Some(&group));
        }
    }

    impl WidgetImpl for SamTimerConfigForm {}
    impl BoxImpl for SamTimerConfigForm {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::prelude::*;

    #[test]
    #[ignore]
    fn test_sam_timer_config_form_layout() {
        gtk::init().expect("Failed to initialize GTK");

        let res_data = include_bytes!(concat!(env!("OUT_DIR"), "/sam_rewritten.gresource"));
        let resource = gtk::gio::Resource::from_data(&glib::Bytes::from_owned(res_data.to_vec()))
            .expect("Failed to load GResource");
        gtk::gio::resources_register(&resource);

        let config_form = SamTimerConfigForm::default();
        config_form.set_margin_top(12);
        config_form.set_margin_bottom(12);
        config_form.set_margin_start(12);
        config_form.set_margin_end(12);

        let config_form_popover = SamTimerConfigForm::default();

        let popover = gtk::Popover::builder().child(&config_form_popover).build();

        let menu_button = gtk::MenuButton::builder()
            .icon_name("emblem-system-symbolic")
            .popover(&popover)
            .build();

        let header_bar = gtk::HeaderBar::builder()
            .title_widget(&gtk::Label::new(Some("Config Form Debugger")))
            .show_title_buttons(true)
            .build();

        header_bar.pack_end(&menu_button);

        let window = gtk::ApplicationWindow::builder()
            .title("SamTimerConfigForm Test")
            .default_width(400)
            .default_height(500)
            .titlebar(&header_bar)
            .child(&config_form)
            .build();

        window.present();

        let context = gtk::glib::MainContext::default();
        while window.is_visible() {
            context.iteration(true);
        }
    }
}
