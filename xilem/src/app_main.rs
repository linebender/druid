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

use std::any::Any;

use druid_shell::{
    kurbo::Size, Application, Cursor, HotKey, Menu, MouseEvent, Region, SysMods, WinHandler,
    WindowBuilder, WindowHandle,
};

use crate::{app::App, widget::RawEvent, View, Widget};

// This is a bit of a hack just to get a window launched. The real version
// would deal with multiple windows and have other ways to configure things.
pub struct AppLauncher<T, V: View<T>, F: FnMut(&mut T) -> V> {
    title: String,
    app: App<T, V, F>,
}

// The logic of this struct is mostly parallel to DruidHandler in win_handler.rs.
struct MainState<T, V: View<T>, F: FnMut(&mut T) -> V>
where
    V::Element: Widget,
{
    handle: WindowHandle,
    app: App<T, V, F>,
}

const QUIT_MENU_ID: u32 = 0x100;

impl<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V + 'static> AppLauncher<T, V, F> {
    pub fn new(app: App<T, V, F>) -> Self {
        AppLauncher {
            title: "Xilem app".into(),
            app,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn run(self) {
        let mut file_menu = Menu::new();
        file_menu.add_item(
            QUIT_MENU_ID,
            "E&xit",
            Some(&HotKey::new(SysMods::Cmd, "q")),
            true,
            false,
        );
        let mut menubar = Menu::new();
        menubar.add_dropdown(Menu::new(), "Application", true);
        menubar.add_dropdown(file_menu, "&File", true);
        let druid_app = Application::new().unwrap();
        let mut builder = WindowBuilder::new(druid_app.clone());
        let main_state = MainState::new(self.app);
        builder.set_handler(Box::new(main_state));
        builder.set_title(self.title);
        builder.set_menu(menubar);
        let window = builder.build().unwrap();
        window.show();
        druid_app.run(None);
    }
}

impl<T: 'static, V: View<T> + 'static, F: FnMut(&mut T) -> V + 'static> WinHandler
    for MainState<T, V, F>
where
    V::Element: Widget,
{
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        self.app.connect(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut druid_shell::piet::Piet, _: &Region) {
        self.app.paint(piet);
    }

    fn command(&mut self, id: u32) {
        match id {
            QUIT_MENU_ID => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseDown(event.into()));
        self.handle.invalidate();
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseUp(event.into()));
        self.handle.invalidate();
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseMove(event.into()));
        self.handle.invalidate();
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        self.app.window_event(RawEvent::MouseWheel(event.into()));
        self.handle.invalidate();
    }

    fn size(&mut self, size: Size) {
        self.app.size(size);
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

impl<T, V: View<T>, F: FnMut(&mut T) -> V> MainState<T, V, F>
where
    V::Element: Widget,
{
    fn new(app: App<T, V, F>) -> Self {
        let state = MainState {
            handle: Default::default(),
            app,
        };
        state
    }
}
