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

extern crate direct2d;
extern crate druid_win_shell;

use std::any::Any;
use std::cell::RefCell;

use direct2d::brush::SolidColorBrush;
use direct2d::math::*;
use direct2d::RenderTarget;

use druid_win_shell::dialog::{FileDialogOptions, FileDialogType};
use druid_win_shell::menu::Menu;
use druid_win_shell::paint::PaintCtx;
use druid_win_shell::win_main;
use druid_win_shell::window::{MouseEvent, WinHandler, WindowBuilder, WindowHandle};

#[derive(Default)]
struct HelloState {
    handle: RefCell<WindowHandle>,
}

impl WinHandler for HelloState {
    fn connect(&self, handle: &WindowHandle) {
        *self.handle.borrow_mut() = handle.clone();
    }

    fn paint(&self, paint_ctx: &mut PaintCtx) -> bool {
        let rt = paint_ctx.render_target();
        let size = rt.get_size();
        let rect = RectF::from((0.0, 0.0, size.width, size.height));
        let bg = SolidColorBrush::create(rt)
            .with_color(0x272822)
            .build()
            .unwrap();
        let fg = SolidColorBrush::create(rt)
            .with_color(0xf0f0ea)
            .build()
            .unwrap();
        rt.fill_rectangle(rect, &bg);
        rt.draw_line((10.0, 50.0), (90.0, 90.0), &fg, 1.0, None);
        false
    }

    fn command(&self, id: u32) {
        match id {
            0x100 => self.handle.borrow().close(),
            0x101 => {
                let mut options = FileDialogOptions::default();
                options.set_show_hidden();
                let filename = self
                    .handle
                    .borrow()
                    .file_dialog(FileDialogType::Open, options);
                println!("result: {:?}", filename);
            }
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

    fn mouse_wheel(&self, delta: i32, mods: u32) {
        println!("mouse_wheel {} {:02x}", delta, mods);
    }

    fn mouse_hwheel(&self, delta: i32, mods: u32) {
        println!("mouse_hwheel {} {:02x}", delta, mods);
    }

    fn mouse_move(&self, x: i32, y: i32, mods: u32) {
        println!("mouse_move ({}, {}) {:02x}", x, y, mods);
    }

    fn mouse(&self, event: &MouseEvent) {
        println!("mouse {:?}", event);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &Any {
        self
    }
}

fn main() {
    druid_win_shell::init();

    let mut file_menu = Menu::new();
    file_menu.add_item(0x100, "E&xit");
    file_menu.add_item(0x101, "O&pen");
    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "&File");

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    builder.set_handler(Box::new(HelloState::default()));
    builder.set_title("Hello example");
    builder.set_menu(menubar);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
