// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An example showing how to change the mouse cursor.
//! Clicking the button should switch your cursor, and
//! the last cursor should be a custom image. Custom
//! image cursors cannot be created before the window is
//! open so we have to work around that. When we receive the
//! `WindowConnected` command we initiate the cursor.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::{
    AppLauncher, Color, Cursor, CursorDesc, Data, Env, ImageBuf, Lens, LocalizedString, WidgetExt,
    WindowDesc,
};

use druid::widget::prelude::*;
use druid::widget::{Button, Controller};

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
        if let Event::WindowConnected = event {
            data.custom = ctx.window().make_cursor(&data.custom_desc);
        }
        child.event(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if data.cursor != old_data.cursor {
            ctx.set_cursor(&data.cursor);
        }
        child.update(ctx, old_data, data, env);
    }
}

fn ui_builder() -> impl Widget<AppState> {
    Button::new("Change cursor")
        .on_click(|_ctx, data: &mut AppState, _env| {
            data.next_cursor();
        })
        .padding(50.0)
        .controller(CursorArea {})
        .border(Color::WHITE, 1.0)
        .padding(50.0)
}

#[derive(Clone, Data, Lens)]
struct AppState {
    cursor: Cursor,
    custom: Option<Cursor>,
    // To see what #[data(ignore)] does look at the docs.rs page on `Data`:
    // https://docs.rs/druid/latest/druid/trait.Data.html
    #[data(ignore)]
    custom_desc: CursorDesc,
}

impl AppState {
    fn next_cursor(&mut self) {
        self.cursor = match self.cursor {
            Cursor::Arrow => Cursor::IBeam,
            Cursor::IBeam => Cursor::Pointer,
            Cursor::Pointer => Cursor::Crosshair,
            Cursor::Crosshair => Cursor::NotAllowed,
            Cursor::NotAllowed => Cursor::ResizeLeftRight,
            Cursor::ResizeLeftRight => Cursor::ResizeUpDown,
            Cursor::ResizeUpDown => {
                if let Some(custom) = &self.custom {
                    custom.clone()
                } else {
                    Cursor::Arrow
                }
            }
            Cursor::Custom(_) => Cursor::Arrow,
            _ => Cursor::Arrow,
        };
    }
}

pub fn main() {
    let main_window =
        WindowDesc::new(ui_builder()).title(LocalizedString::new("Blocking functions"));
    let cursor_image = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();
    // The (0,0) refers to where the "hotspot" is located, so where the mouse actually points.
    // (0,0) is the top left, and (cursor_image.width(), cursor_image.width()) the bottom right.
    let custom_desc = CursorDesc::new(cursor_image, (0.0, 0.0));

    let data = AppState {
        cursor: Cursor::Arrow,
        custom: None,
        custom_desc,
    };
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}
