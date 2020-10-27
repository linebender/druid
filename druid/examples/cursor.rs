// Copyright 2020 The Druid Authors.
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

//! An example showing how to change the mouse cursor.
//! Clicking the button should switch your cursor, and
//! the last cursor should be a custom image. Custom
//! image cursors cannot be created before the window is
//! open so we have to work around that. When we receive the
//! `WindowConnected` command we initiate the cursor.

use druid::{
    AppLauncher, Color, Cursor, CursorDesc, Data, Env, ImageBuf, Lens, LocalizedString, WidgetExt,
    WindowDesc,
};

use druid::widget::prelude::*;
use druid::widget::{Button, Controller};

use std::rc::Rc;

/// This Controller switches the current cursor based on the selection.
/// The crucial part of this code is actually making and initialising
/// the cursor. This happens here. Because we cannot make the cursor
/// before the window is open we have to do that on `WindowConnected`.
struct CursorArea;

impl<W: Widget<AppState>> Controller<AppState, W> for CursorArea {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::WindowConnected => {
                data.custom = ctx.window().make_cursor(&data.custom_desc).map(Rc::new);
            }
            Event::MouseMove(_) => {
                // Because the cursor is reset to the default on every `MouseMove`
                // event we have to explicitly overwrite this every event.
                ctx.set_cursor(&data.cursor);
            }
            _ => {}
        }
        child.event(ctx, event, data, env);
    }
}

fn ui_builder() -> impl Widget<AppState> {
    Button::new("Change cursor")
        .on_click(|ctx, data: &mut AppState, _env| {
            data.cursor = next_cursor(&data.cursor, data.custom.clone());
            // After changing the cursor, we need to update the active cursor
            // via the context in order for the change to take effect immediately.
            ctx.set_cursor(&data.cursor);
        })
        .padding(50.0)
        .controller(CursorArea {})
        .border(Color::WHITE, 1.0)
        .padding(50.0)
}

#[derive(Clone, Data, Lens)]
struct AppState {
    cursor: Rc<Cursor>,
    custom: Option<Rc<Cursor>>,
    custom_desc: Rc<CursorDesc>,
}

fn next_cursor(c: &Cursor, custom: Option<Rc<Cursor>>) -> Rc<Cursor> {
    Rc::new(match c {
        Cursor::Arrow => Cursor::IBeam,
        Cursor::IBeam => Cursor::Crosshair,
        Cursor::Crosshair => Cursor::OpenHand,
        Cursor::OpenHand => Cursor::NotAllowed,
        Cursor::NotAllowed => Cursor::ResizeLeftRight,
        Cursor::ResizeLeftRight => Cursor::ResizeUpDown,
        Cursor::ResizeUpDown => {
            if let Some(custom) = custom {
                return custom;
            } else {
                Cursor::Arrow
            }
        }
        Cursor::Custom(_) => Cursor::Arrow,
    })
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title(LocalizedString::new("Blocking functions"));
    let cursor_image = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();
    // The (0,0) refers to where the "hotspot" is located, so where the mouse actually points.
    // (0,0) is the top left, and (cursor_image.width(), cursor_image.width()) the bottom right.
    let custom_desc = Rc::new(CursorDesc::new(cursor_image, (0.0, 0.0)));

    let data = AppState {
        cursor: Rc::new(Cursor::Arrow),
        custom: None,
        custom_desc,
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}
