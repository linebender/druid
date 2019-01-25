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

//! Sample GUI app.

extern crate direct2d;
extern crate directwrite;
extern crate druid;
extern crate druid_win_shell;

use direct2d::brush::SolidColorBrush;
use direct2d::RenderTarget;

use druid_win_shell::menu::Menu;
use druid_win_shell::win_main;
use druid_win_shell::window::WindowBuilder;

use druid::widget::{Button, Padding, Row};
use druid::{FileDialogOptions, FileDialogType};
use druid::{Ui, UiMain, UiState};

use druid::widget::Widget;
use druid::{BoxConstraints, Geometry, LayoutResult};
use druid::{Id, LayoutCtx, PaintCtx};

const COMMAND_EXIT: u32 = 0x100;
const COMMAND_OPEN: u32 = 0x101;

/// A very simple custom widget.
struct FooWidget;

impl Widget for FooWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let rt = paint_ctx.render_target();
        let fg = SolidColorBrush::create(rt)
            .with_color(0xf0f0ea)
            .build()
            .unwrap();
        let (x, y) = geom.pos;
        rt.draw_line((x, y), (x + geom.size.0, y + geom.size.1), &fg, 1.0, None);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }
}

impl FooWidget {
    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

fn main() {
    druid_win_shell::init();

    let mut file_menu = Menu::new();
    file_menu.add_item(COMMAND_EXIT, "E&xit");
    file_menu.add_item(COMMAND_OPEN, "O&pen");
    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "&File");

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let foo1 = FooWidget.ui(&mut state);
    let foo1 = Padding::uniform(10.0).ui(foo1, &mut state);
    let foo2 = FooWidget.ui(&mut state);
    let foo2 = Padding::uniform(10.0).ui(foo2, &mut state);
    let button = Button::new("Press me").ui(&mut state);
    let buttonp = Padding::uniform(10.0).ui(button, &mut state);
    let button2 = Button::new("Don't press me").ui(&mut state);
    let button2p = Padding::uniform(10.0).ui(button2, &mut state);
    let root = Row::new().ui(&[foo1, foo2, buttonp, button2p], &mut state);
    state.set_root(root);
    state.add_listener(button, move |_: &mut bool, mut ctx| {
        println!("click");
        ctx.poke(button2, &mut "You clicked it!".to_string());
    });
    state.add_listener(button2, move |_: &mut bool, mut ctx| {
        ctx.poke(button2, &mut "Naughty naughty".to_string());
    });
    state.set_command_listener(|cmd, mut ctx| match cmd {
        COMMAND_EXIT => ctx.close(),
        COMMAND_OPEN => {
            let options = FileDialogOptions::default();
            let result = ctx.file_dialog(FileDialogType::Open, options);
            println!("result = {:?}", result);
        }
        _ => println!("unexpected command {}", cmd),
    });
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Hello example");
    builder.set_menu(menubar);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
