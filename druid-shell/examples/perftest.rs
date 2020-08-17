// Copyright 2018 The Druid Authors.
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

use time::Instant;

use piet_common::kurbo::{Line, Rect, Size};
use piet_common::{Color, FontBuilder, Piet, RenderContext, Text, TextLayoutBuilder};

use druid_shell::{Application, KeyEvent, WinHandler, WindowBuilder, WindowHandle};

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);
const FG_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xea);
const RED: Color = Color::rgb8(0xff, 0x80, 0x80);
const CYAN: Color = Color::rgb8(0x80, 0xff, 0xff);

struct PerfTest {
    handle: WindowHandle,
    size: Size,
    start_time: Instant,
    last_time: Instant,
    red: bool,
}

impl WinHandler for PerfTest {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn paint(&mut self, piet: &mut Piet, _: Rect) -> bool {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);

        piet.stroke(
            Line::new((0.0, self.size.height), (self.size.width, 0.0)),
            &FG_COLOR,
            1.0,
        );

        let current_ns = (Instant::now() - self.start_time).whole_nanoseconds();
        let th = ::std::f64::consts::PI * (current_ns as f64) * 2e-9;
        let dx = 100.0 * th.sin();
        let dy = 100.0 * th.cos();
        piet.stroke(
            Line::new((100.0, 100.0), (100.0 + dx, 100.0 - dy)),
            &FG_COLOR,
            1.0,
        );

        let font = piet
            .text()
            .new_font_by_name("Consolas", 15.0)
            .build()
            .unwrap();
        let large_font = piet
            .text()
            .new_font_by_name("Consolas", 48.0)
            .build()
            .unwrap();

        let now = Instant::now();
        let msg = format!("{}ms", (now - self.last_time).whole_milliseconds());
        self.last_time = now;
        let layout = piet
            .text()
            .new_text_layout(&font, &msg, std::f64::INFINITY)
            .build()
            .unwrap();
        piet.draw_text(&layout, (10.0, 210.0), &FG_COLOR);

        let msg = "VSYNC";
        let layout = piet
            .text()
            .new_text_layout(&large_font, &msg, std::f64::INFINITY)
            .build()
            .unwrap();
        let color = if self.red { &RED } else { &CYAN };
        piet.draw_text(&layout, (10.0, 280.0), color);
        self.red = !self.red;

        let msg = "Hello DWrite! This is a somewhat longer string of text intended to provoke slightly longer draw times.";
        let layout = piet
            .text()
            .new_text_layout(&font, &msg, std::f64::INFINITY)
            .build()
            .unwrap();
        let dy = 15.0;
        let x0 = 210.0;
        let y0 = 10.0;
        for i in 0..60 {
            let y = y0 + (i as f64) * dy;
            piet.draw_text(&layout, (x0, y), &FG_COLOR);
        }

        true
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => self.handle.close(),
            _ => println!("unexpected id {}", id),
        }
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        println!("keydown: {:?}", event);
        false
    }

    fn size(&mut self, size: Size) {
        self.size = size;
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
    let perf_test = PerfTest {
        size: Size::ZERO,
        handle: Default::default(),
        start_time: time::Instant::now(),
        last_time: time::Instant::now(),
        red: true,
    };
    builder.set_handler(Box::new(perf_test));
    builder.set_title("Performance tester");

    let window = builder.build().unwrap();
    window.show();

    app.run(None);
}
