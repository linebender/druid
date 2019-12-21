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

//! The fundamental druid types.

use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use crate::kurbo::{Rect, Size};
use crate::piet::Piet;
use crate::{BaseState, Command, Cursor, Text, TimerToken, WinCtx, WindowHandle, WindowId};

/// A context passed to paint methods of widgets.
///
/// Widgets paint their appearance by calling methods on the
/// `render_ctx`, which PaintCtx derefs to for convenience.
/// This struct is expected to grow, for example to include the
/// "damage region" indicating that only a subset of the entire
/// widget hierarchy needs repainting.
pub struct PaintCtx<'a, 'b: 'a> {
    /// The render context for actually painting.
    pub render_ctx: &'a mut Piet<'b>,
    pub window_id: WindowId,
    /// The currently visible region.
    pub(crate) region: Region,
}

/// A region of a widget, generally used to describe what needs to be drawn.
#[derive(Debug, Clone)]
pub struct Region(Rect);

impl Region {
    /// Returns the smallest `Rect` that encloses the entire region.
    pub fn to_rect(&self) -> Rect {
        self.0
    }

    /// Returns `true` if `self` intersects with `other`.
    #[inline]
    pub fn intersects(&self, other: Rect) -> bool {
        self.0.intersect(other).area() > 0.
    }
}

impl From<Rect> for Region {
    fn from(src: Rect) -> Region {
        Region(src)
    }
}

impl<'a, 'b: 'a> Deref for PaintCtx<'a, 'b> {
    type Target = Piet<'b>;

    fn deref(&self) -> &Self::Target {
        self.render_ctx
    }
}

impl<'a, 'b: 'a> DerefMut for PaintCtx<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_ctx
    }
}

impl<'a, 'b: 'a> PaintCtx<'a, 'b> {
    /// Returns the currently visible [`Region`].
    ///
    /// [`Region`]: struct.Region.html
    #[inline]
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// Creates a temporary `PaintCtx` with a new visible region, and calls
    /// the provided function with that `PaintCtx`.
    ///
    /// This is used by containers to ensure that their children have the correct
    /// visible region given their layout.
    pub fn with_child_ctx(&mut self, region: impl Into<Region>, f: impl FnOnce(&mut PaintCtx)) {
        let PaintCtx {
            render_ctx,
            window_id,
            ..
        } = self;
        let mut child_ctx = PaintCtx {
            render_ctx,
            window_id: *window_id,
            region: region.into(),
        };
        f(&mut child_ctx)
    }
}

/// A context provided to layout handling methods of widgets.
///
/// As of now, the main service provided is access to a factory for
/// creating text layout objects, which are likely to be useful
/// during widget layout.
pub struct LayoutCtx<'a, 'b: 'a> {
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) window_id: WindowId,
}

/// A mutable context provided to event handling methods of widgets.
///
/// Widgets should call [`invalidate`] whenever an event causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct EventCtx<'a, 'b> {
    // Note: there's a bunch of state that's just passed down, might
    // want to group that into a single struct.
    pub(crate) win_ctx: &'a mut dyn WinCtx<'b>,
    pub(crate) cursor: &'a mut Option<Cursor>,
    /// Commands submitted to be run after this event.
    pub(crate) command_queue: &'a mut VecDeque<(WindowId, Command)>,
    pub(crate) window_id: WindowId,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    pub(crate) window: &'a WindowHandle,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) had_active: bool,
    pub(crate) is_handled: bool,
    pub(crate) is_root: bool,
}

/// A mutable context provided to data update methods of widgets.
///
/// Widgets should call [`invalidate`] whenever a data change causes a change
/// in the widget's appearance, to schedule a repaint.
///
/// [`invalidate`]: #method.invalidate
pub struct UpdateCtx<'a, 'b: 'a> {
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) window: &'a WindowHandle,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure). But for
    // now keep it super-simple.
    pub(crate) needs_inval: bool,
    pub(crate) window_id: WindowId,
}

impl<'a, 'b> EventCtx<'a, 'b> {
    /// Invalidate.
    ///
    /// Right now, it just invalidates the entire window, but we'll want
    /// finer grained invalidation before long.
    pub fn invalidate(&mut self) {
        // Note: for the current functionality, we could shortcut and just
        // request an invalidate on the window. But when we do fine-grained
        // invalidation, we'll want to compute the invalidation region, and
        // that needs to be propagated (with, likely, special handling for
        // scrolling).
        self.base_state.needs_inval = true;
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        self.win_ctx.text_factory()
    }

    /// Set the cursor icon.
    ///
    /// Call this when handling a mouse move event, to set the cursor for the
    /// widget. A container widget can safely call this method, then recurse
    /// to its children, as a sequence of calls within an event propagation
    /// only has the effect of the last one (ie no need to worry about
    /// flashing).
    ///
    /// This method is expected to be called mostly from the [`MouseMoved`]
    /// event handler, but can also be called in response to other events,
    /// for example pressing a key to change the behavior of a widget.
    ///
    /// [`MouseMoved`]: enum.Event.html#variant.MouseDown
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        *self.cursor = Some(cursor.clone());
    }

    /// Set the "active" state of the widget.
    ///
    /// See [`BaseState::is_active`](struct.BaseState.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// Query the "hot" state of the widget.
    ///
    /// See [`BaseState::is_hot`](struct.BaseState.html#method.is_hot).
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot()
    }

    /// Query the "active" state of the widget.
    ///
    /// This is the same state set by [`set_active`](#method.set_active) and
    /// is provided as a convenience.
    pub fn is_active(&self) -> bool {
        self.base_state.is_active
    }

    /// Returns a reference to the current `WindowHandle`.
    ///
    /// Note: we're in the process of migrating towards providing functionality
    /// provided by the window handle in mutable contexts instead. If you're
    /// considering a new use of this method, try adding it to `WinCtx` and
    /// plumbing it through instead.
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Set the event as "handled", which stops its propagation to other
    /// widgets.
    pub fn set_handled(&mut self) {
        self.is_handled = true;
    }

    /// Determine whether the event has been handled by some other widget.
    pub fn is_handled(&self) -> bool {
        self.is_handled
    }

    /// Query the focus state of the widget.
    ///
    /// See [`BaseState::has_focus`](struct.BaseState.html#method.has_focus).
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus()
    }

    /// Request keyboard focus.
    ///
    /// Discussion question: is method needed in contexts other than event?
    pub fn request_focus(&mut self) {
        self.base_state.request_focus = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.base_state.request_anim = true;
    }

    /// Request a timer event.
    ///
    /// The return value is a token, which can be used to associate the
    /// request with the event.
    pub fn request_timer(&mut self, deadline: Instant) -> TimerToken {
        self.base_state.request_timer = true;
        self.win_ctx.request_timer(deadline)
    }

    /// Returns the layout size of the current widget.
    pub fn size(&self) -> Size {
        self.base_state.size()
    }

    /// Submit a [`Command`] to be run after this event is handled.
    ///
    /// Commands are run in the order they are submitted; all commands
    /// submitted during the handling of an event are executed before
    /// the [`update()`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update()`]: trait.Widget.html#tymethod.update
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        window_id: impl Into<Option<WindowId>>,
    ) {
        let window_id = window_id.into().unwrap_or(self.window_id);
        self.command_queue.push_back((window_id, command.into()))
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}

impl<'a, 'b> LayoutCtx<'a, 'b> {
    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        &mut self.text_factory
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}

impl<'a, 'b> UpdateCtx<'a, 'b> {
    /// Invalidate.
    ///
    /// See [`EventCtx::invalidate`](struct.EventCtx.html#method.invalidate) for
    /// more discussion.
    pub fn invalidate(&mut self) {
        self.needs_inval = true;
    }

    /// Get an object which can create text layouts.
    pub fn text(&mut self) -> &mut Text<'b> {
        self.text_factory
    }

    /// Returns a reference to the current `WindowHandle`.
    ///
    /// Note: For the most part we're trying to migrate `WindowHandle`
    /// functionality to `WinCtx`, but the update flow is the exception, as
    /// it's shared across multiple windows.
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }
}
