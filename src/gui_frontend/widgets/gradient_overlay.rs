use gtk::glib;

glib::wrapper! {
    pub struct GradientOverlay(ObjectSubclass<imp::GradientOverlay>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for GradientOverlay {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use super::*;
    use gtk::prelude::{SnapshotExt, WidgetExt};
    use gtk::subclass::prelude::*;
    #[derive(Default)]
    pub struct GradientOverlay;
    #[glib::object_subclass]
    impl ObjectSubclass for GradientOverlay {
        const NAME: &'static str = "GradientOverlay";
        type Type = super::GradientOverlay;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for GradientOverlay {}
    impl WidgetImpl for GradientOverlay {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let width = self.obj().width() as f32;
            let height = self.obj().height() as f32;
            let stops = [
                gtk::gsk::ColorStop::new(0.0, gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                gtk::gsk::ColorStop::new(0.3, gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                gtk::gsk::ColorStop::new(0.6, gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.6)),
                // gtk::gsk::ColorStop::new(1.0, gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.6)),
            ];
            snapshot.append_linear_gradient(
                &gtk::graphene::Rect::new(0.0, 0.0, width, height),
                &gtk::graphene::Point::new(0.0, 0.0),
                &gtk::graphene::Point::new(0.0, height),
                &stops,
            );
        }
    }
}
