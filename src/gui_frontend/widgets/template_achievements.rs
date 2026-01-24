use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::glib;
use gtk::prelude::{Cast, CastNone, ListModelExt, PopoverExt, SettingsExt, WidgetExt};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use crate::gui_frontend::widgets::template_timer_form::SamTimerConfigForm;

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
        self.sort_store_manually();
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

    pub fn sort_store_manually(&self) {
        let settings = crate::gui_frontend::gsettings::get_settings();
        let method = settings.string("timed-sort-method");

        crate::dev_println!("[CLIENT] Sorting store manually, method: {}", method);

        self.imp().store.sort(move |obj1, obj2| {
            let a = obj1.downcast_ref::<GAchievementObject>().unwrap();
            let b = obj2.downcast_ref::<GAchievementObject>().unwrap();

            if method == "unlock" {
                let a_val = a.global_achieved_percent();
                let b_val = b.global_achieved_percent();
                a_val
                    .partial_cmp(&b_val)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .into()
            } else {
                // Default to A-Z
                let a_name = a.name();
                let b_name = b.name();
                a_name.cmp(&b_name).into()
            }
        });
    }

    pub fn apply_auto_selection_count(&self, count: u32) {
        let store = &self.imp().store;
        let mut selected_count = 0;

        crate::dev_println!("[CLIENT] Applying auto_selection count of {}", count);

        for i in 0..store.n_items() {
            if let Some(obj) = store.item(i).and_downcast::<GAchievementObject>() {
                if !obj.is_achieved() && obj.permission() == 0 {
                    obj.set_is_selected(false);
                }
            }
        }

        for i in 0..store.n_items() {
            if selected_count >= count {
                break;
            }

            if let Some(obj) = store.item(i).and_downcast::<GAchievementObject>() {
                // Check criteria: Not achieved and permission == 0
                if !obj.is_achieved() && obj.permission() == 0 {
                    obj.set_is_selected(true);
                    selected_count += 1;
                }
            }
        }
    }

    pub fn apply_auto_selection_percent(&self, percent: u32) {
        let store = &self.imp().store;
        let percent = percent.clamp(0, 100);

        // 1. Reset all selectable items first
        for i in 0..store.n_items() {
            if let Some(obj) = store.item(i).and_downcast::<GAchievementObject>() {
                if !obj.is_achieved() && obj.permission() == 0 {
                    obj.set_is_selected(false);
                }
            }
        }

        // 2. Count the total pool of selectable achievements
        let mut selectable_pool = Vec::new();
        for i in 0..store.n_items() {
            if let Some(obj) = store.item(i).and_downcast::<GAchievementObject>() {
                if !obj.is_achieved() && obj.permission() == 0 {
                    selectable_pool.push(obj);
                }
            }
        }

        // 3. Calculate target count based on percentage
        let total_selectable = selectable_pool.len() as f64;
        let target_count = ((percent as f64 / 100.0) * total_selectable).round() as usize;

        crate::dev_println!(
            "[CLIENT] Applying auto_selection: {}% of {} items (Target: {})",
            percent,
            total_selectable,
            target_count
        );

        // 4. Select items up to the target count
        for obj in selectable_pool.into_iter().take(target_count) {
            obj.set_is_selected(true);
        }
    }

    pub fn update_selected_label(&self) {
        let imp = self.imp();
        let store = &imp.store;
        let mut count = 0;

        for i in 0..store.n_items() {
            if let Some(obj) = store.item(i).and_downcast::<GAchievementObject>() {
                // Count items that are selected but not yet achieved
                if obj.is_selected() && !obj.is_achieved() {
                    count += 1;
                }
            }
        }

        // Navigate to the child widgets inside the popover [cite: 10]
        if let Some(popover) = imp.config_button.popover().and_downcast::<gtk::Popover>() {
            if let Some(timer_form) = popover.child().and_downcast::<SamTimerConfigForm>() {
                let timer_imp = timer_form.imp();

                // 1. Update the label text
                timer_imp.selected_count_label.set_label(&format!("{} selected", count));

                // 2. Disable/Enable the button based on the count
                timer_imp.start_button.set_sensitive(count > 0);
            }
        }
    }
}

mod imp {
    use gtk::prelude::ListModelExt;
    use crate::gui_frontend::gobjects::achievement::GAchievementObject;
    use crate::gui_frontend::widgets::template_achievement_row::SamAchievementRow;
    use gtk::glib;
    use gtk::prelude::{
        Cast, CastNone, GObjectPropertyExpressionExt, ListItemExt, ObjectExt, ToValue,
        ToggleButtonExt, WidgetExt,
    };
    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, TemplateChild};

    #[derive(CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievements.ui")]
    pub struct SamAchievementsPage {
        #[template_child]
        pub manual_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub timed_mode_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub manual_mode_sort_az_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub manual_mode_sort_unlock_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub config_button: TemplateChild<gtk::MenuButton>,

        pub store: gtk::gio::ListStore,
        pub sort_model: gtk::SortListModel,
    }

    impl Default for SamAchievementsPage {
        fn default() -> Self {
            let store = gtk::gio::ListStore::new::<GAchievementObject>();
            let sort_model = gtk::SortListModel::new(Some(store.clone()), None::<gtk::Sorter>);

            Self {
                manual_mode_btn: TemplateChild::default(),
                timed_mode_btn: TemplateChild::default(),
                list_view: TemplateChild::default(),
                manual_mode_sort_az_btn: TemplateChild::default(),
                manual_mode_sort_unlock_btn: TemplateChild::default(),
                config_button: TemplateChild::default(),
                store,
                sort_model,
            }
        }
    }

    impl SamAchievementsPage {
        fn apply_sorting(&self) {
            // 1. Timed Mode active -> No sorting
            if self.timed_mode_btn.is_active() {
                self.sort_model.set_sorter(None::<&gtk::Sorter>);
                return;
            }

            // 2. A-Z Mode active -> StringSorter
            if self.manual_mode_sort_az_btn.is_active() {
                let sorter = gtk::StringSorter::builder()
                    .expression(GAchievementObject::this_expression("name"))
                    .build();
                self.sort_model.set_sorter(Some(&sorter));
            }
            // 3. Unlock Mode active -> CustomSorter for f32
            else if self.manual_mode_sort_unlock_btn.is_active() {
                let sorter = gtk::CustomSorter::new(move |obj1, obj2| {
                    let a = obj1.downcast_ref::<GAchievementObject>().unwrap();
                    let b = obj2.downcast_ref::<GAchievementObject>().unwrap();

                    let a_val = a.global_achieved_percent();
                    let b_val = b.global_achieved_percent();

                    a_val
                        .partial_cmp(&b_val)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .into()
                });
                self.sort_model.set_sorter(Some(&sorter));
            }
            // 4. Fallback -> No sorting
            else {
                self.sort_model.set_sorter(None::<&gtk::Sorter>);
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

            factory.connect_setup(glib::clone!(
                #[weak]
                obj,
                move |_, list_item| {
                    let row = SamAchievementRow::new();
                    row.setup_expressions();

                    obj.imp().timed_mode_btn.bind_property("active", &row, "select-layout")
                        .sync_create()
                        .build();

                    list_item.set_child(Some(&row));

                    let drag_source = gtk::DragSource::new();
                    let drop_target =
                        gtk::DropTarget::new(glib::Type::OBJECT, gtk::gdk::DragAction::MOVE);
                    drag_source.set_actions(gtk::gdk::DragAction::MOVE);

                    drag_source.connect_prepare(glib::clone!(
                        #[weak]
                        list_item,
                        #[weak]
                        obj,
                        #[upgrade_or]
                        None,
                        move |_, _, _| {
                            if obj.imp().manual_mode_btn.is_active() {
                                return None;
                            }
                            let item = list_item.item()?;
                            Some(gtk::gdk::ContentProvider::for_value(&item.to_value()))
                        }
                    ));

                    drop_target.connect_drop(glib::clone!(
                        #[weak]
                        list_item,
                        #[weak]
                        obj,
                        #[upgrade_or]
                        false,
                        move |_, value, _, _| {
                            let dragged_item = value.get::<GAchievementObject>().ok();
                            let target_item = list_item.item().and_downcast::<GAchievementObject>();

                            if let (Some(source), Some(target)) = (dragged_item, target_item) {
                                obj.move_item(&source, &target);
                                return true;
                            }
                            false
                        }
                    ));

                    row.add_controller(drag_source);
                    row.add_controller(drop_target);
                }
            ));

            factory.connect_bind(glib::clone!(#[weak] obj, move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().expect("Must be ListItem");
                let row = list_item.child().and_downcast::<SamAchievementRow>().expect("Must be SamAchievementRow");

                row.bind(list_item);
            }));

            factory.connect_unbind(move |_, list_item| {
                let row = list_item.child().and_downcast::<SamAchievementRow>().expect("Must be SamAchievementRow");
                row.unbind();
            });

            self.list_view.set_factory(Some(&factory));

            let selection_model = gtk::MultiSelection::new(Some(self.sort_model.clone()));
            self.list_view.set_model(Some(&selection_model));

            self.manual_mode_sort_az_btn.connect_toggled(glib::clone!(
                #[weak]
                obj,
                move |_| obj.imp().apply_sorting()
            ));

            self.manual_mode_sort_unlock_btn
                .connect_toggled(glib::clone!(
                    #[weak]
                    obj,
                    move |_| obj.imp().apply_sorting()
                ));

            self.timed_mode_btn.connect_toggled(glib::clone!(
                #[weak]
                obj,
                move |_| obj.imp().apply_sorting()
            ));

            // Listen for selection changes on every item added to the store
            self.store.connect_items_changed(glib::clone!(#[weak] obj, move |store, _, _, _| {
                for i in 0..store.n_items() {
                    if let Some(item) = store.item(i).and_downcast::<GAchievementObject>() {
                        // Connect to property notifications
                        item.connect_notify_local(Some("is-selected"), glib::clone!(#[weak] obj, move |_, _| {
                            obj.update_selected_label();
                        }));
                        item.connect_notify_local(Some("is-achieved"), glib::clone!(#[weak] obj, move |_, _| {
                            obj.update_selected_label();
                        }));
                    }
                }
                obj.update_selected_label();
            }));

            obj.imp().apply_sorting();
        }
    }

    impl WidgetImpl for SamAchievementsPage {}
    impl BoxImpl for SamAchievementsPage {}
}
