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

use std::any::Any;
use std::cell::RefCell;

use piet_common::kurbo::{Line, Rect};
use piet_common::{Color, FillRule, RenderContext};

use druid_shell::dialog::{FileDialogOptions, FileDialogType};
use druid_shell::keyboard::KeyEvent;
use druid_shell::keycodes::MenuKey;
use druid_shell::menu::Menu;
use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;
use druid_shell::window::{MouseEvent, WinHandler, WindowHandle};

const BG_COLOR: Color = Color::rgb24(0x27_28_22);
const FG_COLOR: Color = Color::rgb24(0xf0_f0_ea);

#[derive(Default)]
struct HelloState {
    size: RefCell<(f64, f64)>,
    handle: RefCell<WindowHandle>,
}

impl WinHandler for HelloState {
    fn connect(&self, handle: &WindowHandle) {
        *self.handle.borrow_mut() = handle.clone();
    }

    fn paint(&self, rc: &mut piet_common::Piet) -> bool {
        let bg = rc.solid_brush(BG_COLOR);
        let fg = rc.solid_brush(FG_COLOR);
        let (width, height) = *self.size.borrow();
        let rect = Rect::new(0.0, 0.0, width, height);
        rc.fill(rect, &bg, FillRule::NonZero);
        rc.stroke(Line::new((10.0, 50.0), (90.0, 90.0)), &fg, 1.0, None);
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

    fn keydown(&self, event: KeyEvent) -> bool {
        println!("keydown: {:?}", event);
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

    fn size(&self, width: u32, height: u32) {
        let dpi = self.handle.borrow().get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        *self.size.borrow_mut() = (width_f, height_f);
    }

    fn destroy(&self) {
        win_main::request_quit();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn main() {
    druid_shell::init();

    let mut file_menu = Menu::new();
    file_menu.add_item(0x100, "E&xit", MenuKey::std_quit());
    file_menu.add_item(0x101, "O&pen", MenuKey::command('o'));
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
