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

use piet_common::kurbo::{Line, Rect, Vec2};
use piet_common::{Color, FillRule, RenderContext};

use druid_shell::dialog::{FileDialogOptions, FileDialogType};
use druid_shell::keyboard::{KeyEvent, KeyModifiers};
use druid_shell::keycodes::MenuKey;
use druid_shell::menu::Menu;
use druid_shell::platform::WindowBuilder;
use druid_shell::runloop;
use druid_shell::window::{Cursor, MouseEvent, TimerToken, WinCtx, WinHandler, WindowHandle};

const BG_COLOR: Color = Color::rgb24(0x27_28_22);
const FG_COLOR: Color = Color::rgb24(0xf0_f0_ea);

#[derive(Default)]
struct HelloState {
    size: (f64, f64),
    handle: WindowHandle,
}

impl WinHandler for HelloState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn paint(&mut self, piet: &mut piet_common::Piet, _ctx: &mut dyn WinCtx) -> bool {
        let bg = piet.solid_brush(BG_COLOR);
        let fg = piet.solid_brush(FG_COLOR);
        let (width, height) = self.size;
        let rect = Rect::new(0.0, 0.0, width, height);
        piet.fill(rect, &bg, FillRule::NonZero);
        piet.stroke(Line::new((10.0, 50.0), (90.0, 90.0)), &fg, 1.0, None);
        false
    }

    fn command(&mut self, id: u32, _ctx: &mut dyn WinCtx) {
        match id {
            0x100 => self.handle.close(),
            0x101 => {
                let mut options = FileDialogOptions::default();
                options.set_show_hidden();
                let filename = self.handle.file_dialog(FileDialogType::Open, options);
                println!("result: {:?}", filename);
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn key_down(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let id = ctx.request_timer(deadline);
        println!("keydown: {:?}, timer id = {:?}", event, id);
        false
    }

    fn wheel(&mut self, delta: Vec2, mods: KeyModifiers, _ctx: &mut dyn WinCtx) {
        println!("mouse_wheel {:?} {:?}", delta, mods);
    }

    fn mouse_move(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        ctx.set_cursor(&Cursor::Arrow);
        println!("mouse_move {:?}", event);
    }

    fn mouse_down(&mut self, event: &MouseEvent, _ctx: &mut dyn WinCtx) {
        println!("mouse_down {:?}", event);
    }

    fn mouse_up(&mut self, event: &MouseEvent, _ctx: &mut dyn WinCtx) {
        println!("mouse_up {:?}", event);
    }

    fn timer(&mut self, id: TimerToken, _ctx: &mut dyn WinCtx) {
        println!("timer fired: {:?}", id);
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        let dpi = self.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
    }

    fn destroy(&mut self, _ctx: &mut dyn WinCtx) {
        runloop::request_quit();
    }

    fn as_any(&mut self) -> &mut dyn Any {
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

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    builder.set_handler(Box::new(HelloState::default()));
    builder.set_title("Hello example");
    builder.set_menu(menubar);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
