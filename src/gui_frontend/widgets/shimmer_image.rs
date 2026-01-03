// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use gtk::glib;
use gtk::glib::subclass::types::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct ShimmerImage(ObjectSubclass<imp::ShimmerImage>)
        @extends gtk::Widget;
}

impl Default for ShimmerImage {
    fn default() -> Self {
        Self::new()
    }
}

impl ShimmerImage {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("url", None::<String>)
            .build()
    }

    pub fn reset(&self) {
        self.imp().url.borrow_mut().take();
        self.imp().texture.borrow_mut().take();
        self.imp().receiver.borrow_mut().take();
        self.imp().loaded.borrow_mut().take();
        self.imp().failed.set(true);
    }
}

mod imp {
    use glib::Properties;
    use glib::ThreadPool;
    use gtk::gdk::{RGBA, Texture};
    use gtk::glib::{self};
    use gtk::graphene::{Point, Rect, Size};
    use gtk::gsk::{ColorStop, LinearGradientNode, RoundedRect};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};
    use std::hash::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::mpsc::{Receiver, TryRecvError};

    const GRADIENT_WIDTH: f32 = 0.8;
    const BASE_COLOR: RGBA = RGBA::new(0.7, 0.7, 0.7, 1.0);
    const HIGHLIGHT_COLOR: RGBA = RGBA::new(0.8, 0.8, 0.8, 1.0);

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ShimmerImage)]
    pub struct ShimmerImage {
        pub start: Cell<i64>,
        pub current: Cell<i64>,
        #[property(get, set)]
        pub url: RefCell<Option<String>>,
        #[property(get, set)]
        pub loaded: RefCell<Option<String>>,
        pub failed: Cell<bool>,
        pub receiver: RefCell<Option<Receiver<Texture>>>,
        pub texture: RefCell<Option<Texture>>,
        pub texture_size_ratio: Cell<f32>,
        #[property(get, set)]
        pub placeholder_height: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ShimmerImage {
        const NAME: &'static str = "ShimmerImage";
        type Type = super::ShimmerImage;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ShimmerImage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.reset();

            self.texture_size_ratio.set(1.0);
            obj.add_tick_callback(|widget, clock| {
                if let Some(this) = widget.downcast_ref::<super::ShimmerImage>() {
                    //Enabling this will cause some of the images to retain their old texture
                    //even if the url property changes, but only if the widget was rendered before
                    //and then jumps into view while it's contents are still cached.
                    //if this.imp().texture.borrow().is_none() {
                    this.queue_draw();
                    //}

                    let imp = this.imp();
                    imp.current.set(clock.frame_time());
                    if imp.start.get() == 0 {
                        imp.start.set(clock.frame_time());
                    }
                }
                glib::ControlFlow::Continue
            });
        }
    }

    impl WidgetImpl for ShimmerImage {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            // This tells GTK that height depends on width
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let ratio = self.texture_size_ratio.get();
            let request_width = self.obj().width_request();
            let request_height = self.obj().height_request();

            if orientation == gtk::Orientation::Horizontal {
                // How wide should we be?
                let width = if for_size < 0 {
                    request_width
                } else {
                    // Height is known, calculate Width: W = H * ratio
                    (for_size as f32 * ratio) as i32
                };

                // Ensure we never report a natural size smaller than our minimum request
                let min_w = request_width.max(16);
                let nat_w = width.max(min_w);

                (min_w, nat_w, -1, -1)
            } else {
                // How tall should we be?
                if self.loaded.borrow().is_none() {
                    let min_h = request_height.max(16);
                    let placeholder_h = self.placeholder_height.get();
                    let nat_h = if placeholder_h > 0 {
                        placeholder_h
                    } else {
                        min_h
                    };
                    return (min_h, nat_h, -1, -1);
                }

                let height = if for_size < 0 {
                    (request_width as f32 / ratio) as i32
                } else if request_width > 0 && for_size > request_width {
                    request_height
                } else {
                    (for_size as f32 / ratio) as i32
                };

                let min_h = request_height.max(16);
                let nat_h = height.max(min_h);

                (min_h, nat_h, -1, -1)
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;

            let rect = Rect::new(0.0, 0.0, width, height);
            let size = Size::new(5.0, 5.0);
            let rounded = RoundedRect::new(rect, size, size, size, size);
            snapshot.push_rounded_clip(&rounded);

            if let Some(url) = self.url.borrow_mut().take()
                && Some(url.as_str()) != self.loaded.borrow().as_deref()
            {
                self.texture.borrow_mut().take();
                self.loaded.borrow_mut().take();
                self.receiver.borrow_mut().take();
                self.load(url.as_str());
                self.loaded.borrow_mut().replace(url);
            }

            let receiver = self.receiver.borrow_mut().take();
            if let Some(receiver) = receiver {
                match receiver.try_recv() {
                    Ok(texture) => {
                        let width = texture.width();
                        let height = texture.height();
                        let ratio = width as f32 / height as f32;
                        self.texture_size_ratio.set(ratio);

                        self.texture.borrow_mut().replace(texture);

                        let obj = self.obj();
                        obj.queue_resize();
                    }
                    Err(TryRecvError::Empty) => {
                        self.receiver.borrow_mut().replace(receiver);
                    }
                    Err(TryRecvError::Disconnected) => {
                        self.failed.set(true);
                    }
                }
            }

            if self.failed.get() {
                // TODO: Insert an icon in the middle: insert-image-symbolic
                snapshot.append_color(&BASE_COLOR, &rect);
            } else if let Some(texture) = &*self.texture.borrow() {
                snapshot.append_texture(texture, &rect);
            } else {
                // convert from continuous microseconds to relative seconds
                let progress =
                    ((self.current.get() - self.start.get()) / 1000) as f32 / 1000.0 % 1.0;
                let progress = ease_in_out(progress);
                let start_pos = -GRADIENT_WIDTH + (1.0 + 2.0 * GRADIENT_WIDTH) * progress;
                let end_pos = start_pos + GRADIENT_WIDTH;

                let color_stops = vec![
                    ColorStop::new(0.0, BASE_COLOR),
                    ColorStop::new(0.3, HIGHLIGHT_COLOR),
                    ColorStop::new(0.5, HIGHLIGHT_COLOR),
                    ColorStop::new(0.7, HIGHLIGHT_COLOR),
                    ColorStop::new(1.0, BASE_COLOR),
                ];

                let gradient = LinearGradientNode::new(
                    &rect,
                    &Point::new(width * start_pos, 0.0),
                    &Point::new(width * end_pos, 0.0),
                    color_stops.as_slice(),
                );
                snapshot.append_node(&gradient);
            }

            fn ease_in_out(t: f32) -> f32 {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            snapshot.pop();
        }
    }

    impl ShimmerImage {
        // TODO: Once we use GTK4.10+, maybe refactor to use MainContext::channel
        // TODO: clear the Pool when the application has requested to exit
        fn load(&self, url: &str) {
            self.failed.set(false);
            let url_string = url.to_string();
            let (sender, receiver) = std::sync::mpsc::channel::<Result<Vec<u8>, ()>>();
            let obj_weak = self.obj().downgrade();

            // Limit concurrency to 8 threads to keep the system smooth
            // This is only meant to be called on the main thread
            thread_local! {
                static POOL: ThreadPool = ThreadPool::shared(Some(8)).expect("Failed to create thread pool");
            }

            POOL.with(|pool| {
                let res = pool.push(move || {
                    let result = if url_string.starts_with("file://") {
                        let path = url_string.replace("file://", "");
                        std::fs::read(path).map_err(|_| ())
                    } else {
                        let mut hasher = DefaultHasher::new();
                        url_string.hash(&mut hasher);
                        let hash_name = format!("{:x}.cache", hasher.finish());

                        let mut cache_path = std::env::temp_dir();
                        cache_path.push(hash_name);

                        if cache_path.exists() {
                            Ok(std::fs::read(&cache_path).unwrap())
                        } else {
                            // Download from HTTPS
                            let client = reqwest::blocking::Client::new();
                            match client.get(&url_string).send().and_then(|res| res.bytes()) {
                                Ok(bytes) => {
                                    let data = bytes.to_vec();
                                    let _ = std::fs::write(&cache_path, &data);
                                    Ok(data)
                                }
                                Err(_) => Err(()),
                            }
                        }
                    };

                    let _ = sender.send(result);

                    // Wake up the Main Loop
                    glib::idle_add(|| glib::ControlFlow::Break);
                });

                if res.is_err() {
                    self.failed.set(true);
                }
            });

            // 3. UI Thread: Watch for the result
            glib::idle_add_local(move || {
                if let Ok(res) = receiver.try_recv() {
                    if let Some(obj) = obj_weak.upgrade() {
                        let imp = obj.imp();
                        match res {
                            Ok(data) => {
                                let gbytes = glib::Bytes::from(&data);
                                if let Ok(texture) = Texture::from_bytes(&gbytes) {
                                    let ratio = texture.width() as f32 / texture.height() as f32;
                                    imp.texture_size_ratio.set(ratio);
                                    imp.texture.borrow_mut().replace(texture);
                                    obj.queue_resize();
                                } else {
                                    imp.failed.set(true);
                                    obj.queue_draw();
                                }
                            }
                            Err(_) => {
                                imp.failed.set(true);
                                obj.queue_draw();
                            }
                        }
                    }
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
        }
    }
}
