use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::glib;
use gtk::prelude::ListModelExt;
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

    pub fn move_item(&self, item: &GAchievementObject, target: &GAchievementObject) {
        let store = &self.imp().store;
        let mut source_pos = None;
        let mut target_pos = None;

        // 1. Find the current positions of the objects
        for i in 0..store.n_items() {
            if let Some(obj) = store.item(i) {
                if &obj == item {
                    source_pos = Some(i);
                }
                if &obj == target {
                    target_pos = Some(i);
                }
            }
        }

        // 2. Perform the move
        if let (Some(src), Some(dst)) = (source_pos, target_pos) {
            if src != dst {
                // If moving downwards, the index shifts after removal,
                // but ListStore::insert handles the logic correctly
                // if you remove then insert.
                let obj = store.item(src).unwrap();
                store.remove(src);
                store.insert(dst, &obj);
            }
        }
    }
}

mod imp {
    use crate::gui_frontend::gobjects::achievement::GAchievementObject;
    use crate::gui_frontend::widgets::template_achievement_row::SamAchievementRow;
    use gtk::glib;
    use gtk::prelude::{CastNone, EventControllerExt, ListItemExt, ObjectExt, RecentManagerExt, StaticType, ToValue, ToggleButtonExt, WidgetExt};
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
            let obj = self.obj();
            let factory = gtk::SignalListItemFactory::new();

            factory.connect_setup(glib::clone!(#[weak] obj, move |_, list_item| {
                let row = SamAchievementRow::new();
                list_item.set_child(Some(&row));

                let drag_source = gtk::DragSource::new();
                let drop_target = gtk::DropTarget::new(glib::Type::OBJECT, gtk::gdk::DragAction::MOVE);
                drag_source.set_actions(gtk::gdk::DragAction::MOVE);

                drag_source.connect_prepare(glib::clone!(#[weak] list_item, #[weak] obj, #[upgrade_or] None, move |_, _, _| {
                    if obj.imp().manual_mode_btn.is_active() {
                        return None;
                    }
                    let item = list_item.item()?;
                    Some(gtk::gdk::ContentProvider::for_value(&item.to_value()))
                }));

                drop_target.connect_drop(glib::clone!(#[weak] list_item, #[weak] obj, #[upgrade_or] false, move |_, value, _, _| {
                    let dragged_item = value.get::<GAchievementObject>().ok();
                    let target_item = list_item.item().and_downcast::<GAchievementObject>();

                    if let (Some(source), Some(target)) = (dragged_item, target_item) {
                        obj.move_item(&source, &target);
                        return true;
                    }
                    false
                }));

                row.add_controller(drag_source);
                row.add_controller(drop_target);
            }));

            factory.connect_bind(glib::clone!(#[weak] obj, move |_, list_item| {
                let item = list_item.item().expect("Item must exist");
                let row = list_item.child().and_downcast::<SamAchievementRow>().expect("Must be SamAchievementRow");

                row.bind(&item);

                obj.imp().timed_mode_btn.bind_property("active", &row, "select-layout")
                    .sync_create()
                    .build();
            }));

            // TODO: connect_unbind

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
