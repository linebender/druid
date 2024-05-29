// Copyright 2022 The Druid Authors.
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

mod app;
mod event;
mod id;
mod view;
mod view_tuple;
mod widget;

use std::any::Any;

use app::App;
use druid_shell::kurbo::Size;
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, Cursor, HotKey, Menu, MouseEvent, Region, SysMods, WinHandler, WindowBuilder,
    WindowHandle,
};
use view::adapt::Adapt;
use view::any_view::AnyView;
use view::button::Button;
use view::column::Column;
use view::memoize::Memoize;
use view::View;
use widget::Widget;

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

struct MainState<T, V: View<T, ()>, F: FnMut(&mut T) -> V>
where
    V::Element: Widget,
{
    size: Size,
    handle: WindowHandle,
    app: App<T, V, F>,
}

impl<T: 'static, V: View<T, ()> + 'static, F: FnMut(&mut T) -> V + 'static> WinHandler
    for MainState<T, V, F>
where
    V::Element: Widget,
{
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut druid_shell::piet::Piet, _: &Region) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);
        self.app.paint(piet);
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn mouse_move(&mut self, _event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        self.app.mouse_down(event.pos);
        self.handle.invalidate();
    }

    fn mouse_up(&mut self, _event: &MouseEvent) {}

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

impl<T, V: View<T, ()>, F: FnMut(&mut T) -> V> MainState<T, V, F>
where
    V::Element: Widget,
{
    fn new(app: App<T, V, F>) -> Self {
        let state = MainState {
            size: Default::default(),
            handle: Default::default(),
            app,
        };
        state
    }
}

/*
fn app_logic(data: &mut u32) -> impl View<u32, (), Element = impl Widget> {
    let button = Button::new(format!("count: {}", data), |data| *data += 1);
    let boxed: Box<dyn AnyView<u32, ()>> = Box::new(button);
    Column::new((boxed, Button::new("reset", |data| *data = 0)))
}
*/

#[derive(Default)]
struct AppData {
    count: u32,
}

fn count_button(count: u32) -> impl View<u32, (), Element = impl Widget> {
    Button::new(format!("count: {}", count), |data| *data += 1)
}

fn app_logic(data: &mut AppData) -> impl View<AppData, (), Element = impl Widget> {
    Column::new((
        Button::new(format!("count: {}", data.count), |data: &mut AppData| {
            data.count += 1
        }),
        Button::new("reset", |data: &mut AppData| data.count = 0),
        Memoize::new(data.count, |count| {
            Button::new(format!("count: {}", count), |data: &mut AppData| {
                data.count += 1
            })
        }),
        Adapt::new(
            |data: &mut AppData, thunk| thunk.call(&mut data.count),
            count_button(data.count),
        ),
    ))
}

fn main() {
    //tracing_subscriber::fmt().init();
    let mut file_menu = Menu::new();
    file_menu.add_item(
        0x100,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        true,
        false,
    );
    let mut menubar = Menu::new();
    menubar.add_dropdown(Menu::new(), "Application", true);
    menubar.add_dropdown(file_menu, "&File", true);

    let app = App::new(AppData::default(), app_logic);
    let druid_app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(druid_app.clone());
    let main_state = MainState::new(app);
    builder.set_handler(Box::new(main_state));
    builder.set_title("Idiopath");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    window.show();

    druid_app.run(None);
}
