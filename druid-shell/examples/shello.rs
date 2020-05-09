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

use druid_shell::kurbo::{Line, Rect};
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, Cursor, FileDialogOptions, FileSpec, HotKey, KeyEvent, Menu, MouseEvent, SysMods,
    TimerToken, WinHandler, WindowBuilder, WindowHandle,
};

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);
const FG_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xea);

#[derive(Default)]
struct HelloState {
    size: (f64, f64),
    handle: WindowHandle,
}

impl WinHandler for HelloState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn paint(&mut self, piet: &mut piet_common::Piet, _: Rect) -> bool {
        let (width, height) = self.size;
        let rect = Rect::new(0.0, 0.0, width, height);
        piet.fill(rect, &BG_COLOR);
        piet.stroke(Line::new((10.0, 50.0), (90.0, 90.0)), &FG_COLOR, 1.0);
        false
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::global().quit()
            }
            0x101 => {
                let options = FileDialogOptions::new().show_hidden().allowed_types(vec![
                    FileSpec::new("Rust Files", &["rs", "toml"]),
                    FileSpec::TEXT,
                    FileSpec::JPG,
                ]);
                let filename = self.handle.open_file_sync(options);
                println!("result: {:?}", filename);
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        let deadline = std::time::Duration::from_millis(500);
        let id = self.handle.request_timer(deadline);
        println!("keydown: {:?}, timer id = {:?}", event, id);

        false
    }

    fn wheel(&mut self, event: &MouseEvent) {
        println!("mouse_wheel {:?}", event);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
        println!("mouse_move {:?}", event);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        println!("mouse_down {:?}", event);
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        println!("mouse_up {:?}", event);
    }

    fn timer(&mut self, id: TimerToken) {
        println!("timer fired: {:?}", id);
    }

    fn size(&mut self, width: u32, height: u32) {
        let dpi = self.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() {
    let mut file_menu = Menu::new();
    file_menu.add_item(
        0x100,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        true,
        false,
    );
    file_menu.add_item(
        0x101,
        "O&pen",
        Some(&HotKey::new(SysMods::Cmd, "o")),
        true,
        false,
    );
    let mut menubar = Menu::new();
    menubar.add_dropdown(Menu::new(), "Application", true);
    menubar.add_dropdown(file_menu, "&File", true);

    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    builder.set_handler(Box::new(HelloState::default()));
    builder.set_title("Hello example");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    window.show();

    app.run(None);
}
