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
    use crate::dev_println;
    use glib::Properties;
    use gtk::gdk::{RGBA, Texture};
    use gtk::gio::spawn_blocking;
    use gtk::glib::{self, Bytes, base64_encode};
    use gtk::graphene::{Point, Rect, Size};
    use gtk::gsk::{ColorStop, LinearGradientNode, RoundedRect};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use reqwest::blocking::Client;
    use std::cell::{Cell, RefCell};
    use std::env::temp_dir;
    use std::fs::{exists, write};
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

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
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;

            let rect = Rect::new(0.0, 0.0, width, height);
            let size = Size::new(5.0, 5.0);
            let rounded = RoundedRect::new(rect, size, size, size, size);
            snapshot.push_rounded_clip(&rounded);

            if let Some(url) = self.url.borrow_mut().take() {
                if Some(url.as_str()) != self.loaded.borrow().as_deref() {
                    self.texture.borrow_mut().take();
                    self.loaded.borrow_mut().take();
                    self.receiver.borrow_mut().take();
                    self.load(url.as_str());
                    self.loaded.borrow_mut().replace(url);
                }
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
                } else {
                    if request_width > 0 && for_size > request_width {
                        request_height
                    } else {
                        (for_size as f32 / ratio) as i32
                    }
                };

                let min_h = request_height.max(16);
                let nat_h = height.max(min_h);

                (min_h, nat_h, -1, -1)
            }
        }
    }

    impl ShimmerImage {
        fn load(&self, url: &str) {
            self.failed.set(false);
            let split = url.split("://").collect::<Vec<&str>>();
            if split.len() != 2 {
                dev_println!("[CLIENT] Invalid URL: {url}");
                self.failed.set(true);
                return;
            }

            let (sender, receiver) = sync_channel::<Texture>(0);
            self.receiver.borrow_mut().replace(receiver);
            let failed = self.failed.clone();

            match split[0] {
                "https" => {
                    let mut path = temp_dir();
                    let url = url.to_string();
                    path.push(format!("{}.jpg", base64_encode(url.as_bytes())));

                    spawn_blocking(move || {
                        if !exists(path.as_path()).unwrap_or_default() {
                            dev_println!("[CLIENT] Downloading: {url}");
                            //Download and store to path
                            let response = match Client::new()
                                .get(url.as_str())
                                .send()
                                .and_then(|response| response.error_for_status())
                                .and_then(|response| response.bytes())
                            {
                                Ok(response) => response,
                                Err(error) => {
                                    failed.set(true);
                                    return eprintln!("[CLIENT] Failed to download {url}: {error}");
                                }
                            };

                            if let Err(error) = write(path.as_path(), response) {
                                failed.set(true);
                                eprintln!("[CLIENT] Failed to write {url} to {path:?}: {error}");
                                return;
                            }
                        } else {
                            dev_println!("[CLIENT] Cached loading: {url}");
                        }

                        let data = match std::fs::read(path.as_path()) {
                            Ok(data) => data,
                            Err(error) => {
                                failed.set(true);
                                eprintln!("[CLIENT] Failed to read {url} from {path:?}: {error}");
                                return;
                            }
                        };

                        match Texture::from_bytes(&Bytes::from(data.as_slice())) {
                            Ok(texture) => {
                                sender.send(texture).ok();
                            }
                            Err(error) => {
                                failed.set(true);
                                eprintln!("[CLIENT] Failed to create {url} from bytes: {error}");
                            }
                        }
                    });
                }
                "file" => {
                    let file_path = split[1].to_string();
                    spawn_blocking(move || {
                        // std::thread::sleep(std::time::Duration::from_millis(5000));
                        let data = match std::fs::read(&file_path) {
                            Ok(data) => data,
                            Err(error) => {
                                failed.set(true);
                                eprintln!("[CLIENT] Failed to read {file_path}: {error}");
                                return;
                            }
                        };

                        match Texture::from_bytes(&Bytes::from(data.as_slice())) {
                            Ok(texture) => {
                                sender.send(texture).ok();
                            }
                            Err(error) => {
                                failed.set(true);
                                eprintln!(
                                    "[CLIENT] Failed to create {file_path} from bytes: {error}"
                                );
                            }
                        }
                    });
                }
                _ => {
                    failed.set(true);
                    dev_println!("[CLIENT] Unsupported URL scheme: {url}");
                }
            }
        }
    }
}
