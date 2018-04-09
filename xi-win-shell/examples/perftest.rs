// Copyright 2018 Google LLC
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

extern crate xi_win_shell;
extern crate direct2d;
extern crate directwrite;
extern crate time;

use std::cell::RefCell;

use time::get_time;

use direct2d::math::*;
use direct2d::render_target::DrawTextOption;
use directwrite::text_format;

use xi_win_shell::paint::PaintCtx;
use xi_win_shell::win_main;
use xi_win_shell::window::{WindowBuilder, WindowHandle, WinHandler};

struct PerfTest(RefCell<PerfState>);

struct PerfState {
    handle: WindowHandle,
    last_time: f64,
    dwrite_factory: directwrite::Factory,
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
        let bg = rt.create_solid_color_brush(0x272822,
            &BrushProperties::default()).unwrap();
        rt.fill_rectangle(&rect, &bg);
        let fg = rt.create_solid_color_brush(0xf0f0ea,
            &BrushProperties::default()).unwrap();

        rt.draw_line(&Point2F::from((0.0, size.height)),
            &Point2F::from((size.width, 0.0)),
            &fg, 1.0, None);

        let th = ::std::f32::consts::PI * (get_time().nsec as f32) * 2e-9;
        let dx = 100.0 * th.sin();
        let dy = 100.0 * th.cos();
        rt.draw_line(&Point2F::from((100.0, 100.0)),
            &Point2F::from((100.0 + dx, 100.0 - dy)),
            &fg, 1.0, None);

        let text_format_params = text_format::ParamBuilder::new()
            .size(15.0)
            .family("Consolas")
            .build().unwrap();
        let text_format = state.dwrite_factory.create(text_format_params).unwrap();

        let now = get_time();
        let now = now.sec as f64 + 1e-9 * now.nsec as f64;
        let msg = format!("{:3.1}ms", 1e3 * (now - state.last_time));
        state.last_time = now;
        rt.draw_text(
            &msg,
            &text_format,
            &RectF::from((10.0, 210.0, 100.0, 300.0)),
            &fg,
            &[DrawTextOption::EnableColorFont]
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
                &RectF::from((x0, y, x0 + 900.0, y + 80.0)),
                &fg,
                &[DrawTextOption::EnableColorFont]
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

    fn char(&self, ch: u32) {
        println!("got char 0x{:x}", ch);
    }

    fn keydown(&self, vk_code: i32) {
        println!("got key code 0x{:x}", vk_code);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }
}

fn main() {
    xi_win_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new(run_loop.get_handle());
    let perf_state = PerfState {
        dwrite_factory: directwrite::Factory::new().unwrap(),
        handle: Default::default(),
        last_time: 0.0,
    };
    builder.set_handler(Box::new(PerfTest(RefCell::new(perf_state))));
    builder.set_title("Performance tester");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
