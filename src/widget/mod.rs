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

//! Common widgets.

use crate::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, WidgetInner,
};

use crate::piet::{Color, FontBuilder, Text, TextLayoutBuilder};
use crate::{Piet, RenderContext, Vec2};

mod flex;
pub use crate::widget::flex::{Column, Flex, Row};

const LABEL_TEXT_COLOR: Color = Color::rgba32(0xf0_f0_ea_ff);

pub struct Label {
    text: String,
}

impl Label {
    /// Discussion question: should this return Label or a wrapped
    /// widget (with WidgetBase)?
    pub fn new(text: impl Into<String>) -> Label {
        Label { text: text.into() }
    }

    fn get_layout(&self, rt: &mut Piet, font_size: f32) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &self.text)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl WidgetInner for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _env: &Env) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(LABEL_TEXT_COLOR);

        let geom = base_state.layout_rect;
        let pos = geom.origin() + Vec2::new(0.0, font_size as f64);
        paint_ctx
            .render_ctx
            .draw_text(&text_layout, pos.to_vec2(), &brush);
    }

    fn layout(&mut self, _layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, _env: &Env) -> Size {
        bc.constrain(Size::new(100.0, 17.0))
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, env: &Env) -> Option<Action> {
        None
    }
}

/*

// The widget trait should probably at least get its own file. When it does,
// the following methods should probably go into it:

#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// The location of the event.
    pub pos: Point,
    /// The modifiers, which have the same interpretation as the raw WM message.
    ///
    /// TODO: rationalize this with mouse mods.
    pub mods: u32,
    /// Which mouse button was pressed.
    pub which: MouseButton,
    /// Count of multiple clicks, is 0 for mouse up event.
    pub count: u32,
}

#[derive(Debug, Clone)]
pub enum KeyVariant {
    /// A virtual-key code, same as WM_KEYDOWN message.
    Vkey(i32),
    /// A Unicode character.
    Char(char),
}

*/
