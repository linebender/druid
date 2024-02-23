// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::Any;

use druid_shell::kurbo::{Line, Size};
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, HotKey, Menu, Region, SysMods, WinHandler, WindowBuilder, WindowHandle,
};

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);
const FG_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xea);

#[derive(Default)]
struct QuitState {
    quit_count: u32,
    size: Size,
    handle: WindowHandle,
}

impl WinHandler for QuitState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut piet_common::Piet, _: &Region) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);
        piet.stroke(Line::new((10.0, 50.0), (90.0, 90.0)), &FG_COLOR, 1.0);
    }

    fn size(&mut self, size: Size) {
        self.size = size;
    }

    fn request_close(&mut self) {
        self.quit_count += 1;
        if self.quit_count >= 5 {
            self.handle.close();
        } else {
            tracing::info!("Don't wanna quit");
        }
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() {
    tracing_subscriber::fmt().init();
    let app = Application::new().unwrap();

    let mut file_menu = Menu::new();
    file_menu.add_item(
        0x100,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        None,
        true,
    );

    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "Application", true);

    let mut builder = WindowBuilder::new(app.clone());
    builder.set_handler(Box::<QuitState>::default());
    builder.set_title("Quit example");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    window.show();

    app.run(None);
}
