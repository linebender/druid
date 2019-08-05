// Copyright 2019 The xi-editor Authors.
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

//! An example of a custom drawing widget.

use druid::kurbo::{Affine, BezPath, Point, Rect, Size};

use druid::piet::{
    Color, FillRule, FontBuilder, ImageFormat, InterpolationMode, RenderContext, Text,
    TextLayoutBuilder,
};

use druid::shell::{runloop, WindowBuilder};

use druid::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UiMain, UiState,
    UpdateCtx, Widget,
};

struct CustomWidget;

impl Widget<String> for CustomWidget {
    // The paint method gets called last, after an event flow.
    // It goes event -> update -> layout -> paint, and each method can influence the next.
    // Basically, anything that changes the appearance of a widget causes a paint.
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        base_state: &BaseState,
        data: &String,
        _env: &Env,
    ) {
        // Let's draw a picture with Piet!
        // Clear the whole context with the color of your choice
        paint_ctx.clear(Color::WHITE);

        // Create an arbitrary bezier path
        // (base_state.size() returns the size of the layout rect we're painting in)
        let mut path = BezPath::new();
        path.move_to(Point::ORIGIN);
        path.quad_to(
            (80.0, 90.0),
            (base_state.size().width, base_state.size().height),
        );
        // Create a solid brush
        let brush = paint_ctx.solid_brush(Color::rgb24(0x00_80_00));
        // Stroke the path with the brush, with thickness 1.0
        paint_ctx.stroke(path, &brush, 1.0, None);

        // Rectangles: the path for practical people
        let rect = Rect::from_origin_size((10., 10.), (100., 100.));
        // Note the Color:rgba32 which includes an alpha channel (7F in this case)
        let brush = paint_ctx.solid_brush(Color::rgba32(0x00_00_00_7F));
        // A fill uses a brush, just like stroke, but it needs FillRule to be set
        paint_ctx.fill(rect, &brush, FillRule::NonZero);

        // Text is easy, if you ignore all these unwraps. Just pick a font and a size.
        let font = paint_ctx
            .text()
            .new_font_by_name("Segoe UI", 24.0)
            .unwrap()
            .build()
            .unwrap();
        // Here's where we actually use the UI state
        let layout = paint_ctx
            .text()
            .new_text_layout(&font, data)
            .unwrap()
            .build()
            .unwrap();

        // Let's rotate our text slightly. First we save our current (default) context:
        paint_ctx
            .with_save(|rc| {
                // Now we can rotate the context (or set a clip path, for instance):
                rc.transform(Affine::rotate(0.1));
                rc.draw_text(&layout, (80.0, 40.0), &brush);
                Ok(())
            })
            .unwrap();
        // When we exit with_save, the original context's rotation is restored

        // Let's burn some CPU to make a (partially transparent) image buffer
        let image_data = make_image_data(256, 256);
        let image = paint_ctx
            .make_image(256, 256, &image_data, ImageFormat::RgbaSeparate)
            .unwrap();
        // The image is automatically scaled to fit the rect you pass to draw_image
        paint_ctx.draw_image(
            &image,
            Rect::from_origin_size(Point::ORIGIN, base_state.size()),
            InterpolationMode::Bilinear,
        );
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &String,
        _env: &Env,
    ) -> Size {
        // You can return any Size.
        // Flexible widgets are based on the BoxConstraints passed by their parent widget.
        bc.max()
    }

    fn event(
        &mut self,
        _event: &Event,
        _ctx: &mut EventCtx,
        _data: &mut String,
        _env: &Env,
    ) -> Option<Action> {
        None
    }

    fn update(
        &mut self,
        _ctx: &mut UpdateCtx,
        _old_data: Option<&String>,
        _data: &String,
        _env: &Env,
    ) {
    }
}

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let root = CustomWidget {};

    let state = UiState::new(root, "Druid + Piet".to_string());

    builder.set_title("Custom widget example");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}

fn make_image_data(width: usize, height: usize) -> Vec<u8> {
    let mut result = vec![0; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let ix = (y * width + x) * 4;
            result[ix + 0] = x as u8;
            result[ix + 1] = y as u8;
            result[ix + 2] = !(x as u8);
            result[ix + 3] = 127;
        }
    }
    result
}
