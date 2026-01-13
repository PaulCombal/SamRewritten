use gtk::glib;

glib::wrapper! {
    pub struct SamAchievementsPage(ObjectSubclass<imp::SamAchievementsPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SamAchievementsPage {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use gtk::glib;
    use gtk::{CompositeTemplate, TemplateChild};
    use gtk::prelude::ToggleButtonExt;
    use gtk::subclass::prelude::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievements.ui")]
    pub struct SamAchievementsPage {
        // These names must match the "id" or object name in your Blueprint file
        #[template_child]
        pub manual_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub timed_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SamAchievementsPage {
        const NAME: &'static str = "SamAchievementsPage";
        type Type = super::SamAchievementsPage;
        type ParentType = gtk::Box; // Matches the root of your Blueprint template

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SamAchievementsPage {
        fn constructed(&self) {
            self.parent_constructed();

            // You can set up your mode-switch logic here
            let obj = self.obj();
            self.manual_mode_btn.connect_toggled(glib::clone!(#[weak] obj, move |btn| {
            if btn.is_active() {
                println!("Switching to Manual Mode");
                // logic to refresh list or change models
            }
        }));
        }
    }

    impl WidgetImpl for SamAchievementsPage {}
    impl BoxImpl for SamAchievementsPage {}
}