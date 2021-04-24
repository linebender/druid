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

use std::any::Any;

use druid_shell::kurbo::Size;
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, Cursor, FileDialogOptions, FileDialogToken, FileInfo, FileSpec, HotKey, KeyEvent,
    Menu, MouseEvent, Region, SysMods, WinHandler, WindowBuilder, WindowHandle,
};

mod element;
mod elm;
mod tree;
mod vdom;
mod window;

use crate::element::{Action, Button, ButtonCmd, Element};
use crate::tree::{Id, Mutation, MutationEl};
use crate::vdom::{Reconciler, VdomNode};
use crate::window::Window;

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

struct MainState {
    size: Size,
    handle: WindowHandle,
    window: Window,
}

impl WinHandler for MainState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut druid_shell::piet::Piet, _: &Region) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);
        self.window.paint(piet);
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
                self.handle.open_file(options);
            }
            _ => println!("unexpected id {}", id),
        }
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

    fn mouse_move(&mut self, _event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        self.window.mouse_down(event.pos);
        self.handle.invalidate();
    }

    fn mouse_up(&mut self, _event: &MouseEvent) {}

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
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl MainState {
    fn new(app_logic: impl FnMut(Vec<Action>) -> Mutation + 'static) -> MainState {
        let window = Window::new(Box::new(app_logic));
        let mut state = MainState {
            size: Default::default(),
            handle: Default::default(),
            window,
        };
        state.window.run_app_logic();
        state
    }
}

#[derive(Default)]
struct ManualMutationCount {
    count: usize,
    button_id: Option<Id>,
}

impl ManualMutationCount {
    fn run(&mut self, actions: Vec<Action>) -> Mutation {
        fn mk_button_mut(count: usize) -> Mutation {
            Mutation {
                cmds: Some(Box::new(ButtonCmd::SetText(format!("count: {}", count)))),
                child: vec![],
            }
        }
        if let Some(_button_id) = self.button_id {
            self.count += actions.len();
            Mutation {
                cmds: None,
                child: vec![MutationEl::Update(mk_button_mut(self.count))],
            }
        } else {
            let id = Id::next();
            let button = Button::default();
            let boxed_button: Box<dyn Element> = Box::new(button);
            self.button_id = Some(id);
            Mutation {
                cmds: None,
                child: vec![MutationEl::Insert(
                    id,
                    Box::new(boxed_button),
                    mk_button_mut(self.count),
                )],
            }
        }
    }
}

// Note: this should become a generic implementation.
struct VdomCount {
    reconciler: Reconciler<VdomCountState>,
    state: VdomCountState,
}

#[derive(Default)]
struct VdomCountState {
    count: usize,
}

impl VdomCountState {
    fn render(&self) -> VdomNode<Self> {
        VdomNode::Column(vec![VdomNode::Button(
            format!("count: {}", self.count),
            Box::new(|state: &mut VdomCountState| state.count += 1),
        )])
    }
}

impl VdomCount {
    fn run(&mut self, actions: Vec<Action>) -> Mutation {
        self.reconciler.run_actions(actions, &mut self.state);
        let vdom = self.state.render();
        self.reconciler.reconcile(vdom)
    }
}

#[derive(Default)]
struct ElmCountApp {
    count: usize,
}

enum ElmCountMsg {
    Increment,
}

impl elm::AppLogic for ElmCountApp {
    type Msg = ElmCountMsg;

    fn update(&mut self, msg: Self::Msg) {
        match msg {
            ElmCountMsg::Increment => self.count += 1,
        }
    }

    fn view(&mut self) -> Box<dyn elm::Vdom<ElmCountMsg>> {
        let button = elm::Button::new(format!("count: {}", self.count), |_action| {
            ElmCountMsg::Increment
        });
        let mut column = elm::Column::new();
        column.add_child(button);
        Box::new(column)
    }
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

    //let mut app_logic = ManualMutationCount::default();

    // Vdom implementation
    /*
    let mut app_logic = VdomCount {
        // Note: this id is bogus. It's probably not needed.
        // TODO: change reconciler to not need root id
        reconciler: Reconciler::new(Id::next()),
        state: VdomCountState::default(),
    };
    */

    let elm_app_logic = ElmCountApp::default();
    let mut app_logic = elm::ElmApp::new(elm_app_logic);

    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    let main_state = MainState::new(move |actions| app_logic.run(actions));
    builder.set_handler(Box::new(main_state));
    builder.set_title("Hello example");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    window.show();

    app.run(None);
}
