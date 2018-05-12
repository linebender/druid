// Copyright 2018 Google LLC
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

//! Widget trait and common widgets.

use std::any::Any;

use xi_win_shell::window::{MouseButton, MouseType};

use {BoxConstraints, Geometry, LayoutResult};
use {HandlerCtx, Id, LayoutCtx, PaintCtx};

mod button;
pub use widget::button::{Button, Label};

mod event_forwarder;
pub use widget::event_forwarder::EventForwarder;

mod flex;
pub use widget::flex::{Column, Flex, Row};

mod padding;
pub use widget::padding::Padding;

/// The trait implemented by all widgets.
pub trait Widget {
    /// Paint the widget's appearance into the paint context.
    ///
    /// The implementer is responsible for translating the coordinates as
    /// specified in the geometry.
    #[allow(unused)]
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {}

    /// Participate in the layout protocol.
    ///
    /// `size` is the size of the child previously requested by a RequestChild return.
    ///
    /// The default implementation is suitable for widgets with a single child, and
    /// just forwards the layout unmodified.
    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult
    {
        if let Some(size) = size {
            // Maybe this is not necessary, rely on default value.
            ctx.position_child(children[0], (0.0, 0.0));
            LayoutResult::Size(size)
        } else {
            LayoutResult::RequestChild(children[0], *bc)
        }
    }


    /// Handle a mouse event.
    ///
    /// TODO: `MouseType` will be replaced by a click count.
    #[allow(unused)]
    fn mouse(&mut self, x: f32, y: f32, mods: u32, which: MouseButton, ty: MouseType,
        ctx: &mut HandlerCtx) -> bool
    { false }

    /// An `escape hatch` of sorts for accessing widget state beyond the widget
    /// methods. Returns true if it is handled.
    #[allow(unused)]
    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool { false }
}
