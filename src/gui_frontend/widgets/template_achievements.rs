use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::glib;
use gtk::subclass::prelude::ObjectSubclassIsExt;

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

impl SamAchievementsPage {
    pub fn clear_model(&self) {
        self.imp().store.remove_all();
    }

    pub fn extend_model_from_slice(&self, slice: &[GAchievementObject]) {
        self.imp().store.extend_from_slice(slice);
    }
}

mod imp {
    use crate::gui_frontend::gobjects::achievement::GAchievementObject;
    use crate::gui_frontend::widgets::template_achievement_row::SamAchievementRow;
    use gtk::glib;
    use gtk::prelude::{Cast, ListItemExt, ToggleButtonExt};
    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, TemplateChild};

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievements.ui")]
    pub struct SamAchievementsPage {
        // These names must match the "id" or object name in your Blueprint file
        #[template_child]
        pub manual_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub timed_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,

        pub store: gtk::gio::ListStore,
    }

    impl Default for SamAchievementsPage {
        fn default() -> Self {
            Self {
                manual_mode_btn: TemplateChild::default(),
                timed_mode_btn: TemplateChild::default(),
                list_view: TemplateChild::default(),
                store: gtk::gio::ListStore::new::<GAchievementObject>(),
            }
        }
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

            // 1. Define the Factory
            let factory = gtk::SignalListItemFactory::new();

            // 2. Setup: Create the Row Widget
            factory.connect_setup(move |_, list_item| {
                let row = SamAchievementRow::new();
                list_item.set_child(Some(&row));
            });

            // 3. Bind: Map Data to the Row
            factory.connect_bind(move |_, list_item| {
                let item = list_item.item().expect("Item must exist");
                let row = list_item
                    .child()
                    .expect("Child must exist")
                    .downcast::<SamAchievementRow>()
                    .expect("Child must be SamAchievementRow");

                row.bind(&item);
            });

            // TODO: connect_unbind

            // 4. Attach to the ListView
            self.list_view.set_factory(Some(&factory));

            let selection_model = gtk::MultiSelection::new(Some(self.store.clone()));
            self.list_view.set_model(Some(&selection_model));

            // You can set up your mode-switch logic here
            let obj = self.obj();
            self.manual_mode_btn.connect_toggled(glib::clone!(
                #[weak]
                obj,
                move |btn| {
                    if btn.is_active() {
                        println!("Switching to Manual Mode");
                        // logic to refresh list or change models
                    }
                }
            ));
        }
    }

    impl WidgetImpl for SamAchievementsPage {}
    impl BoxImpl for SamAchievementsPage {}
}
