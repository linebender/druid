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

/// An example that is as simple as possible (just throw up an empty window).
use std::any::Any;

use druid_shell::kurbo::{Point, Rect, Size};
use druid_shell::piet::{Color, FixedLinearGradient, GradientStop, RenderContext};

use druid_shell::{
    Application, Cursor, FileDialogToken, FileInfo, KeyEvent, MouseEvent, Region, TimerToken,
    WinHandler, WindowBuilder, WindowHandle,
};

#[derive(Default)]
struct HelloState {
    size: Size,
    handle: Option<WindowHandle>,
}

impl WinHandler for HelloState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = Some(handle.clone());
    }

    fn prepare_paint(&mut self) {
        self.handle.as_mut().unwrap().invalidate();
    }

    fn paint(&mut self, piet: &mut piet_common::Piet, _: &Region) {
        // draw a gradient so we can see what's going on.
        let brush = piet
            .gradient(FixedLinearGradient {
                start: Point::ZERO,
                end: self.size.to_vec2().to_point(),
                stops: vec![
                    GradientStop {
                        pos: 0.0,
                        color: Color::RED,
                    },
                    GradientStop {
                        pos: 1.0,
                        color: Color::BLUE,
                    },
                ],
            })
            .unwrap();
        piet.fill(Rect::ZERO.with_size(self.size), &brush);
    }

    fn command(&mut self, id: u32) {
        println!("command id {}", id);
    }

    fn open_file(&mut self, _token: FileDialogToken, file_info: Option<FileInfo>) {
        println!("open file result: {:?}", file_info);
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        println!("keydown: {:?}", event);
        false
    }

    fn key_up(&mut self, event: KeyEvent) {
        println!("keyup: {:?}", event);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        println!("mouse_wheel {:?}", event);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.handle.as_mut().unwrap().set_cursor(&Cursor::Arrow);
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

    fn size(&mut self, size: Size) {
        self.size = size;
    }

    fn got_focus(&mut self) {
        println!("Got focus");
    }

    fn lost_focus(&mut self) {
        println!("Lost focus");
    }

    fn request_close(&mut self) {
        self.handle.as_ref().unwrap().close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    builder.set_handler(Box::new(HelloState::default()));
    builder.set_title("Hello example");

    let window = builder.build().unwrap();
    window.show();

    app.run(None);
}
