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
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, TemplateChild};

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/samrewritten/SamRewritten/ui/achievement_timer_form.ui")]
    pub struct SamTimerConfigForm {
        #[template_child]
        pub radio_15: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub radio_30: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub radio_60: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub radio_custom: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub start_session_btn: TemplateChild<gtk::Button>,
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

            // Example: Handling the "Start" button click
            let obj = self.obj();

            self.start_session_btn.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    // Get the implementation pointer inside the closure
                    let imp = obj.imp();

                    let duration = if imp.radio_15.is_active() {
                        15
                    } else if imp.radio_30.is_active() {
                        30
                    } else if imp.radio_60.is_active() {
                        60
                    } else {
                        0
                    };

                    println!("Starting session with {} minutes", duration);

                    if let Some(widget) = obj.ancestor(gtk::Popover::static_type()) {
                        if let Ok(popover) = widget.downcast::<gtk::Popover>() {
                            popover.popdown();
                        }
                    }
                }
            ));
        }
    }

    impl WidgetImpl for SamTimerConfigForm {}
    impl BoxImpl for SamTimerConfigForm {}
}
