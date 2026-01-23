use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct SamAchievementRow(ObjectSubclass<imp::SamAchievementRow>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SamAchievementRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn setup_expressions(&self) {
        let imp = self.imp();

        // The root expression: self.list-item.item (as GAchievementObject)
        let item_expr = self.property_expression("list-item")
            .chain_property::<gtk::ListItem>("item");

        // --- 1. Basic Text & Images ---
        item_expr.clone().chain_property::<GAchievementObject>("name")
            .bind(&imp.name_label.get(), "label", gtk::Widget::NONE);

        item_expr.clone().chain_property::<GAchievementObject>("description")
            .bind(&imp.description_label.get(), "label", gtk::Widget::NONE);

        item_expr.clone().chain_property::<GAchievementObject>("icon-normal")
            .bind(&imp.normal_icon.get(), "url", gtk::Widget::NONE);

        item_expr.clone().chain_property::<GAchievementObject>("icon-locked")
            .bind(&imp.locked_icon.get(), "url", gtk::Widget::NONE);

        // --- 2. Progress Bar ---
        item_expr.clone().chain_property::<GAchievementObject>("global-achieved-percent")
            .bind(&imp.global_percentage_progress_bar.get(), "value", gtk::Widget::NONE);

        item_expr.clone().chain_property::<GAchievementObject>("global-achieved-percent-ok")
            .bind(&imp.global_percentage_progress_bar.get(), "visible", gtk::Widget::NONE);

        // --- 3. Complex Visibility Logic (The fix for your original request) ---

        // Checkbox Visibility: Visible ONLY IF (select_layout == true) AND (is_achieved == false)
        gtk::ClosureExpression::new::<bool>(
            &[
                self.property_expression("select-layout"),
                item_expr.clone().chain_property::<GAchievementObject>("is-achieved"),
            ],
            glib::closure_local!(|_: Option<glib::Object>, layout: bool, achieved: bool| {
                layout && !achieved
            }),
        )
            .bind(&imp.achievement_check.get(), "visible", gtk::Widget::NONE);

        // --- 4. Logic via Chained Closures ---

        // Icon Stack: is-achieved -> "normal" or "locked"
        item_expr.clone().chain_property::<GAchievementObject>("is-achieved")
            .chain_closure::<String>(glib::closure_local!(|_: Option<glib::Object>, achieved: bool| {
                if achieved { "normal" } else { "locked" }
            }))
            .bind(&imp.icon_stack.get(), "visible-child-name", gtk::Widget::NONE);

        // Permission: permission (i32) -> switch sensitivity
        item_expr.clone().chain_property::<GAchievementObject>("permission")
            .chain_closure::<bool>(glib::closure_local!(|_: Option<glib::Object>, perm: i32| perm == 0))
            .bind(&imp.achievement_switch.get(), "sensitive", gtk::Widget::NONE);

        // Permission: permission (i32) -> protected icon visibility
        item_expr.clone().chain_property::<GAchievementObject>("permission")
            .chain_closure::<bool>(glib::closure_local!(|_: Option<glib::Object>, perm: i32| perm != 0))
            .bind(&imp.protected_icon.get(), "visible", gtk::Widget::NONE);

        // Checkbox sensitivity: !is-achieved
        item_expr.clone().chain_property::<GAchievementObject>("is-achieved")
            .chain_closure::<bool>(glib::closure_local!(|_: Option<glib::Object>, achieved: bool| !achieved))
            .bind(&imp.achievement_check.get(), "sensitive", gtk::Widget::NONE);
    }

    pub fn bind(&self, list_item: &gtk::ListItem) {
        let imp = self.imp();
        let item = list_item.item()
            .and_downcast::<GAchievementObject>()
            .expect("Model mismatch");

        // 1. Clear old bindings before starting new ones
        self.unbind();

        // 2. Set the list item (triggers all Expressions)
        self.set_list_item(Some(list_item.clone()));

        // 3. Create bidirectional bindings and store them
        let mut bindings = imp.active_bindings.borrow_mut();

        bindings.push(
            item.bind_property("is-achieved", &imp.achievement_switch.get(), "active")
                .bidirectional()
                .sync_create()
                .build()
        );

        bindings.push(
            item.bind_property("is-selected", &imp.achievement_check.get(), "active")
                .bidirectional()
                .sync_create()
                .build()
        );
    }


    pub fn unbind(&self) {
        let imp = self.imp();

        // 1. Reset the Expressions
        self.set_list_item(None::<gtk::ListItem>);

        // 2. Manually unbind and clear the vector
        let mut bindings = imp.active_bindings.borrow_mut();
        for binding in bindings.drain(..) {
            binding.unbind();
        }
    }

}

mod imp {
    use super::*;
    use crate::gui_frontend::widgets::custom_progress_bar::CustomProgressBar;
    use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
    use gtk::glib::Properties;
    use gtk::{CompositeTemplate, TemplateChild};
    use std::cell::{Cell, RefCell};

    #[derive(Default, CompositeTemplate, Properties)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievement_row.ui")]
    #[properties(wrapper_type = super::SamAchievementRow)]
    pub struct SamAchievementRow {
        #[template_child]
        pub drag_handle: TemplateChild<gtk::Image>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub normal_icon: TemplateChild<ShimmerImage>,
        #[template_child]
        pub locked_icon: TemplateChild<ShimmerImage>,
        #[template_child]
        pub icon_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub achievement_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub achievement_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub protected_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub global_percentage_progress_bar: TemplateChild<CustomProgressBar>,

        #[property(get, set)]
        pub select_layout: Cell<bool>,

        #[property(get, set, nullable)]
        pub list_item: RefCell<Option<gtk::ListItem>>,

        pub active_bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SamAchievementRow {
        const NAME: &'static str = "SamAchievementRow";
        type Type = super::SamAchievementRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SamAchievementRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        // fn constructed(&self) {
        //     self.parent_constructed();
        //     let obj = self.obj();
        //     let imp = obj.imp();
        // }
    }

    impl WidgetImpl for SamAchievementRow {}
    impl BoxImpl for SamAchievementRow {}
}
