// Copyright 2018 The xi-editor Authors.
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

extern crate druid_shell;
extern crate kurbo;
extern crate piet;
extern crate piet_common;
extern crate time;

use std::any::Any;
use std::cell::RefCell;

use time::get_time;

use kurbo::{Line, Rect};
use piet::{FillRule, FontBuilder, RenderContext, Text, TextLayoutBuilder};

use druid_shell::win_main;
use druid_shell::window::{WinHandler, WindowHandle};
use druid_shell::windows::{PresentStrategy, WindowBuilder};

struct PerfTest(RefCell<PerfState>);

struct PerfState {
    handle: WindowHandle,
    size: (f64, f64),
    last_time: f64,
}

impl WinHandler for PerfTest {
    fn connect(&self, handle: &WindowHandle) {
        self.0.borrow_mut().handle = handle.clone();
    }

    fn paint(&self, rc: &mut piet_common::Piet) -> bool {
        let mut state = self.0.borrow_mut();
        let (width, height) = state.size;
        let bg = rc.solid_brush(0x272822ff).unwrap();
        let fg = rc.solid_brush(0xf0f0eaff).unwrap();
        let rect = Rect::new(0.0, 0.0, width, height);
        rc.fill(rect, &bg, FillRule::NonZero);

        rc.stroke(Line::new((0.0, height), (width, 0.0)), &fg, 1.0, None);

        let th = ::std::f64::consts::PI * (get_time().nsec as f64) * 2e-9;
        let dx = 100.0 * th.sin();
        let dy = 100.0 * th.cos();
        rc.stroke(
            Line::new((100.0, 100.0), (100.0 + dx, 100.0 - dy)),
            &fg,
            1.0,
            None,
        );

        let font = rc
            .text()
            .new_font_by_name("Consolas", 15.0)
            .unwrap()
            .build()
            .unwrap();

        let now = get_time();
        let now = now.sec as f64 + 1e-9 * now.nsec as f64;
        let msg = format!("{:3.1}ms", 1e3 * (now - state.last_time));
        state.last_time = now;
        let layout = rc
            .text()
            .new_text_layout(&font, &msg)
            .unwrap()
            .build()
            .unwrap();
        rc.draw_text(&layout, (10.0, 210.0), &fg);

        let msg = "Hello DWrite! This is a somewhat longer string of text intended to provoke slightly longer draw times.";
        let layout = rc
            .text()
            .new_text_layout(&font, &msg)
            .unwrap()
            .build()
            .unwrap();
        let dy = 15.0;
        let x0 = 210.0;
        let y0 = 10.0;
        for i in 0..60 {
            let y = y0 + (i as f32) * dy;
            rc.draw_text(&layout, (x0, y), &fg);
        }

        true
    }

    fn command(&self, id: u32) {
        match id {
            0x100 => self.0.borrow().handle.close(),
            _ => println!("unexpected id {}", id),
        }
    }

    fn char(&self, ch: u32, mods: u32) {
        println!("got char 0x{:x} {:02x}", ch, mods);
    }

    fn keydown(&self, vk_code: i32, mods: u32) -> bool {
        println!("got key code 0x{:x} {:02x}", vk_code, mods);
        false
    }

    fn size(&self, width: u32, height: u32) {
        let mut state = self.0.borrow_mut();
        let dpi = state.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        state.size = (width_f, height_f);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &Any {
        self
    }
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let perf_state = PerfState {
        size: Default::default(),
        handle: Default::default(),
        last_time: 0.0,
    };
    builder.set_handler(Box::new(PerfTest(RefCell::new(perf_state))));
    builder.set_title("Performance tester");
    // Note: experiment with changing this
    builder.set_present_strategy(PresentStrategy::FlipRedirect);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
