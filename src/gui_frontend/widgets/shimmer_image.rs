// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
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

glib::wrapper! {
    pub struct ShimmerImage(ObjectSubclass<imp::ShimmerImage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
        self.set_url("");
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
    use std::collections::HashMap;
    use std::hash::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::OnceLock;
    use std::time::Duration;

    const GRADIENT_WIDTH: f32 = 0.8;
    const BASE_COLOR: RGBA = RGBA::new(0.7, 0.7, 0.7, 1.0);
    const HIGHLIGHT_COLOR: RGBA = RGBA::new(0.8, 0.8, 0.8, 1.0);

    const TEXTURE_CACHE_CAPACITY: usize = 256;

    fn http_client() -> &'static reqwest::blocking::Client {
        static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
        CLIENT.get_or_init(|| {
            reqwest::blocking::Client::builder()
                .pool_idle_timeout(Some(Duration::from_secs(30)))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new())
        })
    }

    struct TextureCache {
        entries: HashMap<String, (Texture, u64)>,
        counter: u64,
        capacity: usize,
    }

    impl TextureCache {
        fn new(capacity: usize) -> Self {
            Self {
                entries: HashMap::new(),
                counter: 0,
                capacity,
            }
        }

        fn get(&mut self, url: &str) -> Option<Texture> {
            let entry = self.entries.get_mut(url)?;
            self.counter += 1;
            entry.1 = self.counter;
            Some(entry.0.clone())
        }

        fn insert(&mut self, url: String, texture: Texture) {
            self.counter += 1;
            if let Some(entry) = self.entries.get_mut(&url) {
                *entry = (texture, self.counter);
                return;
            }
            if self.entries.len() >= self.capacity
                && let Some(evict_key) = self
                    .entries
                    .iter()
                    .min_by_key(|(_, (_, c))| *c)
                    .map(|(k, _)| k.clone())
            {
                self.entries.remove(&evict_key);
            }
            self.entries.insert(url, (texture, self.counter));
        }
    }

    thread_local! {
        static TEXTURE_CACHE: RefCell<TextureCache> =
            RefCell::new(TextureCache::new(TEXTURE_CACHE_CAPACITY));
        static IN_FLIGHT: RefCell<HashMap<String, Vec<glib::WeakRef<super::ShimmerImage>>>> =
            RefCell::new(HashMap::new());
        // Limit concurrency to 8 threads to keep the system smooth
        static POOL: ThreadPool = ThreadPool::shared(Some(8)).expect("Failed to create thread pool");
    }

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::ShimmerImage)]
    pub struct ShimmerImage {
        pub start: Cell<i64>,
        pub current: Cell<i64>,
        #[property(get, set)]
        pub url: RefCell<Option<String>>,
        pub failed: Cell<bool>,
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

            self.texture_size_ratio.set(1.0);
            self.failed.set(true);

            obj.connect_url_notify(|this| {
                let imp = this.imp();
                imp.texture.replace(None);
                imp.texture_size_ratio.set(1.0);
                imp.start.set(0);

                let url_opt = imp.url.borrow().clone();
                match url_opt.as_deref() {
                    Some(url) if !url.is_empty() => {
                        imp.failed.set(false);
                        imp.load(url);
                    }
                    _ => {
                        imp.failed.set(true);
                    }
                }

                this.queue_resize();
            });

            obj.add_tick_callback(|widget, clock| {
                if let Some(this) = widget.downcast_ref::<super::ShimmerImage>() {
                    let imp = this.imp();
                    if imp.texture.borrow().is_none() && !imp.failed.get() {
                        imp.current.set(clock.frame_time());
                        if imp.start.get() == 0 {
                            imp.start.set(clock.frame_time());
                        }
                        this.queue_draw();
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
                let width = if for_size < 0 {
                    request_width
                } else {
                    (for_size as f32 * ratio) as i32
                };

                let min_w = request_width.max(16);
                let nat_w = width.max(min_w);

                (min_w, nat_w, -1, -1)
            } else {
                if self.texture.borrow().is_none() {
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

                let color_stops = [
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
                    &color_stops,
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
        fn apply_texture(&self, texture: Texture) {
            let ratio = texture.width() as f32 / texture.height() as f32;
            self.texture_size_ratio.set(ratio);
            self.texture.replace(Some(texture));
            self.failed.set(false);
            self.obj().queue_resize();
        }

        // TODO: clear the Pool when the application has requested to exit
        fn load(&self, url: &str) {
            if let Some(texture) = TEXTURE_CACHE.with(|c| c.borrow_mut().get(url)) {
                self.apply_texture(texture);
                return;
            }

            let obj_weak = self.obj().downgrade();
            let url_string = url.to_string();
            let already_in_flight = IN_FLIGHT.with(|f| {
                let mut map = f.borrow_mut();
                if let Some(waiters) = map.get_mut(&url_string) {
                    waiters.push(obj_weak);
                    true
                } else {
                    map.insert(url_string.clone(), vec![obj_weak]);
                    false
                }
            });

            if already_in_flight {
                return;
            }

            let url_for_bail = url_string.clone();
            let push_res = POOL.with(|pool| {
                pool.push(move || {
                    let result: Result<Vec<u8>, ()> = if url_string.starts_with("file://") {
                        let path = url_string.replace("file://", "");
                        std::fs::read(path).map_err(|_| ())
                    } else {
                        let mut hasher = DefaultHasher::new();
                        url_string.hash(&mut hasher);
                        let hash_name = format!("{:x}.cache", hasher.finish());

                        let mut cache_path = std::env::temp_dir();
                        cache_path.push(hash_name);

                        if cache_path.exists() {
                            std::fs::read(&cache_path).map_err(|_| ())
                        } else {
                            match http_client()
                                .get(&url_string)
                                .send()
                                .and_then(|res| res.bytes())
                            {
                                Ok(bytes) => {
                                    let data = bytes.to_vec();
                                    let _ = std::fs::write(&cache_path, &data);
                                    Ok(data)
                                }
                                Err(_) => Err(()),
                            }
                        }
                    };

                    glib::MainContext::default().invoke(move || {
                        let waiters = IN_FLIGHT
                            .with(|f| f.borrow_mut().remove(&url_string))
                            .unwrap_or_default();

                        let texture_result: Result<Texture, ()> = result.and_then(|data| {
                            let gbytes = glib::Bytes::from(&data);
                            Texture::from_bytes(&gbytes).map_err(|_| ())
                        });

                        if let Ok(ref texture) = texture_result {
                            TEXTURE_CACHE.with(|c| {
                                c.borrow_mut()
                                    .insert(url_string.clone(), texture.clone())
                            });
                        }

                        for weak in waiters {
                            let Some(obj) = weak.upgrade() else { continue };
                            // Widget may have been recycled to a different URL mid-flight
                            if obj.imp().url.borrow().as_deref() != Some(url_string.as_str()) {
                                continue;
                            }
                            match texture_result {
                                Ok(ref texture) => obj.imp().apply_texture(texture.clone()),
                                Err(_) => {
                                    obj.imp().failed.set(true);
                                    obj.queue_draw();
                                }
                            }
                        }
                    });
                })
            });

            if push_res.is_err() {
                IN_FLIGHT.with(|f| f.borrow_mut().remove(&url_for_bail));
                self.failed.set(true);
            }
        }
    }
}
