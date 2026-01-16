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

    pub fn bind(&self, item: &glib::Object) {
        let imp = self.imp();
        let item = item.downcast_ref::<GAchievementObject>().unwrap();

        // 1. Basic Property Bindings
        item.bind_property("name", &imp.name_label.get(), "label")
            .sync_create()
            .build();

        item.bind_property("description", &imp.description_label.get(), "label")
            .sync_create()
            .build();

        item.bind_property("icon-normal", &imp.normal_icon.get(), "url")
            .sync_create()
            .build();

        item.bind_property("icon-locked", &imp.locked_icon.get(), "url")
            .sync_create()
            .build();

        // Bidirectional: user toggling the switch updates the GObject property
        item.bind_property("is-achieved", &imp.achievement_switch.get(), "active")
            .bidirectional()
            .sync_create()
            .build();

        item.bind_property(
            "global-achieved-percent",
            &imp.global_percentage_progress_bar.get(),
            "value",
        )
        .sync_create()
        .build();

        item.bind_property(
            "global-achieved-percent-ok",
            &imp.global_percentage_progress_bar.get(),
            "visible",
        )
        .sync_create()
        .build();

        // 2. Complex Logic via Expressions
        // Icon Stack: is-achieved (bool) -> visible-child-name (string)
        item.property_expression("is-achieved")
            .chain_closure::<String>(glib::closure_local!(
                |_: Option<glib::Object>, is_achieved: bool| {
                    if is_achieved { "normal" } else { "locked" }
                }
            ))
            .bind(
                &imp.icon_stack.get(),
                "visible-child-name",
                gtk::Widget::NONE,
            );

        // Permission: permission (i32) -> sensitive (bool)
        item.property_expression("permission")
            .chain_closure::<bool>(glib::closure_local!(
                |_: Option<glib::Object>, permission: i32| { permission == 0 }
            ))
            .bind(
                &imp.achievement_switch.get(),
                "sensitive",
                gtk::Widget::NONE,
            );

        // Protected Icon: permission (i32) -> visible (bool)
        item.property_expression("permission")
            .chain_closure::<bool>(glib::closure_local!(
                |_: Option<glib::Object>, permission: i32| { permission != 0 }
            ))
            .bind(&imp.protected_icon.get(), "visible", gtk::Widget::NONE);
    }
}

mod imp {
    use super::*;
    use crate::gui_frontend::widgets::custom_progress_bar::CustomProgressBar;
    use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
    use gtk::{CompositeTemplate, TemplateChild};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievement_row.ui")]
    pub struct SamAchievementRow {
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
        pub protected_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub global_percentage_progress_bar: TemplateChild<CustomProgressBar>,
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

    impl ObjectImpl for SamAchievementRow {}
    impl WidgetImpl for SamAchievementRow {}
    impl BoxImpl for SamAchievementRow {}
}
