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
extern crate piet;
extern crate time;

use std::any::Any;
use std::cell::RefCell;

use time::get_time;

use piet::math::*;
use piet::RenderTarget;
use piet::brush::SolidColorBrush;
use piet::write::{self,TextFormat};

use druid_shell::paint::PaintCtx;
use druid_shell::util::default_text_options;
use druid_shell::win_main;
use druid_shell::window::{PresentStrategy, WindowBuilder, WindowHandle, WinHandler};

struct PerfTest(RefCell<PerfState>);

struct PerfState {
    handle: WindowHandle,
    last_time: f64,
    write_factory: write::Factory,
}

impl WinHandler for PerfTest {
    fn connect(&self, handle: &WindowHandle) {
        self.0.borrow_mut().handle = handle.clone();
    }

    fn paint(&self, paint_ctx: &mut PaintCtx) -> bool {
        let mut state = self.0.borrow_mut();
        let rt = paint_ctx.render_target();
        let size = rt.get_size();
        let rect = RectF::from((0.0, 0.0, size.width, size.height));
        let bg = SolidColorBrush::create(rt).with_color(0x272822).build().unwrap();
        let fg = SolidColorBrush::create(rt).with_color(0xf0f0ea).build().unwrap();
        rt.fill_rectangle(rect, &bg);

        rt.draw_line((0.0, size.height), (size.width, 0.0), &fg, 1.0, None);

        let th = ::std::f32::consts::PI * (get_time().nsec as f32) * 2e-9;
        let dx = 100.0 * th.sin();
        let dy = 100.0 * th.cos();
        rt.draw_line((100.0, 100.0), (100.0 + dx, 100.0 - dy),
            &fg, 1.0, None);

        let text_format = TextFormat::create(&state.write_factory)
            .with_family("Consolas")
            .with_size(15.0)
            .build()
            .unwrap();

        let now = get_time();
        let now = now.sec as f64 + 1e-9 * now.nsec as f64;
        let msg = format!("{:3.1}ms", 1e3 * (now - state.last_time));
        state.last_time = now;
        rt.draw_text(
            &msg,
            &text_format,
            (10.0, 210.0, 100.0, 300.0),
            &fg,
            default_text_options()
        );

        let msg = "Hello DWrite! This is a somewhat longer string of text intended to provoke slightly longer draw times.";
        let dy = 15.0;
        let x0 = 210.0;
        let y0 = 10.0;
        for i in 0..60 {
            let y = y0 + (i as f32) * dy;
            rt.draw_text(
                msg,
                &text_format,
                (x0, y, x0 + 900.0, y + 80.0),
                &fg,
                default_text_options()
            );
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

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &Any { self }
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let perf_state = PerfState {
        write_factory: directwrite::Factory::new().unwrap(),
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
