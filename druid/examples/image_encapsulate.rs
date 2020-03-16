// Copyright 2020 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This example shows how to draw an png image.
//!
//! Requires the non-default "image" feature to be enabled:
//! `cargo run --example image_encapsulate --features "images"`
//!

#[cfg(not(feature = "image"))]
fn main() {
    eprintln!("This examples requires the \"image\" feature to be enabled:");
    eprintln!("cargo run --example image_encapsulate --features \"image\"");
}

#[cfg(feature = "image")]
fn main() {
    use druid::{
        widget::{Flex, Image, ImageData, Slider, WidgetExt},
        AppLauncher, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
        PaintCtx, Size, UpdateCtx, Widget, WindowDesc,
    };

    pub struct ImageSwitcher {
        image_index: u32,
        images: Vec<Box<dyn Widget<u32>>>,
    }

    impl ImageSwitcher {
        pub fn new(newimages: Vec<Box<dyn Widget<u32>>>) -> Self {
            Self {
                image_index: 0,
                images: newimages,
            }
        }
    }

    impl Widget<f64> for ImageSwitcher {
        fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut f64, _env: &Env) {}

        fn lifecycle(
            &mut self,
            _ctx: &mut LifeCycleCtx,
            _event: &LifeCycle,
            _data: &f64,
            _env: &Env,
        ) {
        }

        fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, data: &f64, _env: &Env) {
            let index_float = *data * (self.images.len() as f64);
            self.image_index = (self.images.len() as u32 - 1).min(index_float as u32);
            ctx.request_paint();
        }

        fn layout(
            &mut self,
            layout_ctx: &mut LayoutCtx,
            bc: &BoxConstraints,
            _data: &f64,
            env: &Env,
        ) -> Size {
            self.images[self.image_index as usize].layout(layout_ctx, bc, &0, env)
        }
        fn paint(&mut self, paint_ctx: &mut PaintCtx, _data: &f64, env: &Env) {
            self.images[self.image_index as usize].paint(paint_ctx, &0_u32, env)
        }
    }

    fn ui_builder() -> impl Widget<f64> {
        let png_1 = Box::new(Image::new(
            ImageData::from_file("examples/PicWithAlpha.png").unwrap(),
        ));
        let png_2 = Box::new(Image::new(
            ImageData::from_file("examples/pngexample.png").unwrap(),
        ));
        let png_3 = Box::new(Image::new(
            ImageData::from_file("examples/PicWithAlpha.png").unwrap(),
        ));

        let mut col = Flex::column();

        col.add_flex_child(
            ImageSwitcher::new(vec![png_1, png_2, png_3]).expand_height(),
            1.0,
        );
        col.add_flex_child(Slider::new().expand_width(), 0.);

        col
    };

    let main_window = WindowDesc::new(ui_builder);
    let data = 0.0;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}
