// Copyright 2020 The Druid Authors.
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

use std::any::Any;

use std::time::{Duration, Instant};

use druid_shell::kurbo::{Point, Rect, Size};
use druid_shell::piet::{Color, Piet, RenderContext};

use druid_shell::{Application, Region, TimerToken, WinHandler, WindowBuilder, WindowHandle};

struct InvalidateTest {
    handle: WindowHandle,
    size: Size,
    start_time: Instant,
    color: Color,
    rect: Rect,
}

impl InvalidateTest {
    fn update_color_and_rect(&mut self) {
        let time_since_start = (Instant::now() - self.start_time).as_millis();
        let (r, g, b, _) = self.color.as_rgba8();
        self.color = match (time_since_start % 2, time_since_start % 3) {
            (0, _) => Color::rgb8(r.wrapping_add(10), g, b),
            (_, 0) => Color::rgb8(r, g.wrapping_add(10), b),
            (_, _) => Color::rgb8(r, g, b.wrapping_add(10)),
        };

        self.rect.x0 = (self.rect.x0 + 5.0) % self.size.width;
        self.rect.x1 = (self.rect.x1 + 5.5) % self.size.width;
        self.rect.y0 = (self.rect.y0 + 3.0) % self.size.height;
        self.rect.y1 = (self.rect.y1 + 3.5) % self.size.height;
    }
}

impl WinHandler for InvalidateTest {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        self.handle.request_timer(Duration::from_millis(60));
    }

    fn timer(&mut self, _id: TimerToken) {
        self.update_color_and_rect();
        self.handle.invalidate_rect(self.rect);
        self.handle.request_timer(Duration::from_millis(60));
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet, region: &Region) {
        piet.fill(region.bounding_box(), &self.color);
    }

    fn size(&mut self, size: Size) {
        self.size = size;
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => self.handle.close(),
            _ => println!("unexpected id {}", id),
        }
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() {
    simple_logger::init().expect("Failed to init simple logger");
    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    let inv_test = InvalidateTest {
        size: Size::ZERO,
        handle: Default::default(),
        start_time: Instant::now(),
        rect: Rect::from_origin_size(Point::ZERO, (10.0, 20.0)),
        color: Color::WHITE,
    };
    builder.set_handler(Box::new(inv_test));
    builder.set_title("Invalidate tester");

    let window = builder.build().unwrap();
    window.show();
    app.run(None);
}
