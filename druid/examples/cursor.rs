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

use druid::{
    AppLauncher, Color, Cursor, CursorDesc, Data, Env, ImageBuf, Lens, LocalizedString, Widget,
    WidgetExt, WindowDesc,
};

use druid::widget::prelude::*;
use druid::widget::{Button, Controller};

use std::sync::Arc;

#[derive(Clone, Data, Lens)]
struct AppState {
    cursor: Arc<Cursor>,
    custom_desc: Arc<CursorDesc>,
    custom: Option<Arc<Cursor>>,
}

fn next_cursor(c: &Cursor, custom: &Option<Arc<Cursor>>) -> Cursor {
    match c {
        Cursor::Arrow => Cursor::IBeam,
        Cursor::IBeam => Cursor::Crosshair,
        Cursor::Crosshair => Cursor::OpenHand,
        Cursor::OpenHand => Cursor::NotAllowed,
        Cursor::NotAllowed => Cursor::ResizeLeftRight,
        Cursor::ResizeLeftRight => Cursor::ResizeUpDown,
        Cursor::ResizeUpDown => {
            if let Some(custom) = custom {
                Cursor::clone(&custom)
            } else {
                Cursor::Arrow
            }
        }
        Cursor::Custom(_) => Cursor::Arrow,
    }
}

struct CursorArea {}

impl<W: Widget<AppState>> Controller<AppState, W> for CursorArea {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if data.custom.is_none() {
            data.custom = ctx.window().make_cursor(&data.custom_desc).map(Arc::new);
        }
        if matches!(event, Event::MouseMove(_)) {
            ctx.set_cursor(&data.cursor);
        }
        child.event(ctx, event, data, env);
    }
}

fn ui_builder() -> impl Widget<AppState> {
    Button::new("Change cursor")
        .on_click(|ctx, data: &mut AppState, _env| {
            data.cursor = Arc::new(next_cursor(&data.cursor, &data.custom));
            ctx.set_cursor(&data.cursor);
        })
        .padding(50.0)
        .controller(CursorArea {})
        .border(Color::WHITE, 1.0)
        .padding(50.0)
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title(LocalizedString::new("Blocking functions"));
    let cursor_image = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();
    let custom_desc = CursorDesc::new(cursor_image, (50.0, 50.0));
    let data = AppState {
        cursor: Arc::new(Cursor::Arrow),
        custom: None,
        custom_desc: Arc::new(custom_desc),
    };

    let app = AppLauncher::with_window(main_window);
    app.use_simple_logger().launch(data).expect("launch failed");
}
