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

//! Widget trait and common widgets.

use std::any::Any;

pub use druid_win_shell::window::MouseButton;

use {BoxConstraints, Geometry, LayoutResult};
use {HandlerCtx, Id, LayoutCtx, PaintCtx};

mod button;
pub use widget::button::{Button, Label};

mod event_forwarder;
pub use widget::event_forwarder::EventForwarder;

mod flex;
pub use widget::flex::{Column, Flex, Row};

mod key_listener;
pub use widget::key_listener::KeyListener;

mod null;
pub(crate) use widget::null::NullWidget;

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
    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        if let Some(size) = size {
            // Maybe this is not necessary, rely on default value.
            ctx.position_child(children[0], (0.0, 0.0));
            LayoutResult::Size(size)
        } else {
            LayoutResult::RequestChild(children[0], *bc)
        }
    }

    /// Sent to the widget on mouse event.
    ///
    /// Mouse events are propagated in a post-order traversal of the widget tree,
    /// culled by geometry. Propagation stops as soon as the event is handled.
    #[allow(unused)]
    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        false
    }

    /// Sent to the active or hot widget on mouse move events.
    // TODO: should mods be plumbed here?
    #[allow(unused)]
    fn mouse_moved(&mut self, x: f32, y: f32, ctx: &mut HandlerCtx) {}

    /// Sent to the widget when its "hot" status changes.
    #[allow(unused)]
    fn on_hot_changed(&mut self, hot: bool, ctx: &mut HandlerCtx) {}

    /// An "escape hatch" of sorts for accessing widget state beyond the widget
    /// methods. Returns true if it is handled.
    #[allow(unused)]
    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        false
    }

    /// Sent to the widget on key event.
    ///
    /// Key events are only sent to the focused widget.
    ///
    /// Note that keys that are interpreted as characters are sent twice, first
    /// as a `Vkey`, then as a `Char`.
    ///
    /// This is a fairly thin wrapper over WM messages. Keyboard input will be
    /// changing quite a bit when IME is implemented.
    ///
    /// Returns true if the event is handled.
    #[allow(unused)]
    fn key(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        false
    }

    /// Called at the beginning of a new animation frame.
    ///
    /// The `interval` argument is the time in nanoseconds between frames, for
    /// the purpose of computing animations. When framerate is steady, it should
    /// be exactly the reciprocal of the refresh rate of the monitor. If we are
    /// skipping frames, its cumulative sum should approximately track the
    /// passage of wall clock time and otherwise should be chosen to optimize
    /// for smoothness of animations.
    ///
    /// The ideal heuristic for computing this interval is a deep topic, ideally
    /// the subject of a blog post when I figure it out.
    ///
    /// On the first frame when transitioning from idle to animating, `interval`
    /// will be 0. (This logic is presently per-window but might change to
    /// per-widget to make it more consistent).
    ///
    /// This method should call `invalidate` on the context if the visual display
    /// updates. In the current implementation, that's not very useful, but it
    /// will become much more so when there's fine grained invalidation.
    ///
    /// The method can also call `request_anim_frame` to keep the animation running.
    #[allow(unused)]
    fn anim_frame(&mut self, interval: u64, ctx: &mut HandlerCtx) {}

    /// Called when a child widget is removed.
    #[allow(unused)]
    fn on_child_removed(&mut self, child: Id) {}
}

pub struct MouseEvent {
    /// X coordinate in px units, relative to top left of widget.
    pub x: f32,
    /// Y coordinate in px units, relative to top left of widget.
    pub y: f32,
    /// The modifiers, which have the same interpretation as the raw WM message.
    ///
    /// TODO: rationalize this with mouse mods.
    pub mods: u32,
    /// Which mouse button was pressed.
    pub which: MouseButton,
    /// Count of multiple clicks, is 0 for mouse up event.
    pub count: u32,
}

#[derive(Clone)]
pub struct KeyEvent {
    pub key: KeyVariant,
    /// The modifiers, a combinations of `M_ALT`, `M_CTRL`, `M_SHIFT`.
    pub mods: u32,
}

#[derive(Clone)]
pub enum KeyVariant {
    /// A virtual-key code, same as WM_KEYDOWN message.
    Vkey(i32),
    /// A Unicode character.
    Char(char),
}
