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

use log;

use crate::bloom::Bloom;
use crate::kurbo::{Affine, Rect, Shape, Size};
use crate::piet::{Piet, RenderContext};
use crate::{
    BoxConstraints, Command, Cursor, Data, Env, Event, LifeCycle, Target, Text, TimerToken, Widget,
    WidgetId, WinCtx, WindowHandle, WindowId,
};

/// Convenience type for dynamic boxed widget.
pub type BoxedWidget<T> = WidgetPod<T, Box<dyn Widget<T>>>;

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// This struct also contains the previous data for a widget, which is
/// essential for the [`update`] method, both to decide when the update
/// needs to propagate, and to provide the previous data so that a
/// widget can process a diff between the old value and the new.
///
/// [`update`]: trait.Widget.html#tymethod.update
pub struct WidgetPod<T: Data, W: Widget<T>> {
    state: BaseState,
    old_data: Option<T>,
    env: Option<Env>,
    inner: W,
    id: WidgetId,
}

/// Generic state for all widgets in the hierarchy.
///
/// This struct contains the widget's layout rect, flags
/// indicating when the widget is active or focused, and other
/// state necessary for the widget to participate in event
/// flow.
///
/// It is provided to [`paint`] calls as a non-mutable reference,
/// largely so a widget can know its size, also because active
/// and focus state can affect the widget's appearance. Other than
/// that, widgets will generally not interact with it directly,
/// but it is an important part of the [`WidgetPod`] struct.
///
/// [`paint`]: trait.Widget.html#tymethod.paint
/// [`WidgetPod`]: struct.WidgetPod.html
#[derive(Default)]
pub(crate) struct BaseState {
    layout_rect: Rect,

    // TODO: consider using bitflags for the booleans.

    // This should become an invalidation rect.
    pub(crate) needs_inval: bool,

    is_hot: bool,

    is_active: bool,

    /// Any descendant is active.
    has_active: bool,

    /// Any descendant has requested an animation frame.
    pub(crate) request_anim: bool,

    /// Any descendant has requested a timer.
    ///
    /// Note: we don't have any way of clearing this request, as it's
    /// likely not worth the complexity.
    request_timer: bool,

    /// This widget or a descendant has focus.
    has_focus: bool,

    /// This widget or a descendant has requested focus.
    pub(crate) request_focus: bool,
    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
}

impl<T: Data, W: Widget<T>> WidgetPod<T, W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<T, W> {
        let id = inner.id().unwrap_or_else(WidgetId::next);
        WidgetPod {
            state: BaseState {
                children_changed: true,
                ..Default::default()
            },
            old_data: None,
            env: None,
            inner,
            id,
        }
    }

    /// Query the "active" state of the widget.
    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    /// Returns `true` if any descendant is active.
    pub fn has_active(&self) -> bool {
        self.state.has_active
    }

    /// Query the "hot" state of the widget.
    pub fn is_hot(&self) -> bool {
        self.state.is_hot
    }

    /// Return a reference to the inner widget.
    pub fn widget(&self) -> &W {
        &self.inner
    }

    /// Return a mutable reference to the inner widget.
    pub fn widget_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    /// Get the identity of the widget.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// Set layout rectangle.
    ///
    /// Intended to be called on child widget in container's `layout`
    /// implementation.
    pub fn set_layout_rect(&mut self, layout_rect: Rect) {
        self.state.layout_rect = layout_rect;
    }

    /// Get the layout rectangle.
    ///
    /// This will be same value as set by `set_layout_rect`.
    pub fn get_layout_rect(&self) -> Rect {
        self.state.layout_rect
    }

    /// Paint a child widget.
    ///
    /// Generally called by container widgets as part of their [`paint`]
    /// method.
    ///
    /// Note that this method does not apply the offset of the layout rect.
    /// If that is desired, use [`paint_with_offset`] instead.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    /// [`paint`]: trait.Widget.html#method.paint
    /// [`paint_with_offset`]: #method.paint_with_offset
    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut ctx = PaintCtx {
            render_ctx: paint_ctx.render_ctx,
            window_id: paint_ctx.window_id,
            region: paint_ctx.region.clone(),
            base_state: &self.state,
        };
        self.inner.paint(&mut ctx, data, &env);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    ///
    /// This will recursively paint widgets, stopping if a widget's layout
    /// rect is outside of the currently visible region.
    // Discussion: should this be `paint` and the other `paint_raw`?
    pub fn paint_with_offset(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_with_offset_impl(paint_ctx, data, env, false)
    }

    /// Paint the widget, even if its layout rect is outside of the currently
    /// visible region.
    pub fn paint_with_offset_always(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_with_offset_impl(paint_ctx, data, env, true)
    }

    /// Shared implementation that can skip drawing non-visible content.
    fn paint_with_offset_impl(
        &mut self,
        paint_ctx: &mut PaintCtx,
        data: &T,
        env: &Env,
        paint_if_not_visible: bool,
    ) {
        if !paint_if_not_visible && !paint_ctx.region().intersects(self.state.layout_rect) {
            return;
        }

        if let Err(e) = paint_ctx.save() {
            log::error!("saving render context failed: {:?}", e);
            return;
        }

        let layout_origin = self.state.layout_rect.origin().to_vec2();
        paint_ctx.transform(Affine::translate(layout_origin));

        let visible = paint_ctx.region().to_rect() - layout_origin;

        paint_ctx.with_child_ctx(visible, |ctx| self.paint(ctx, data, &env));

        if let Err(e) = paint_ctx.restore() {
            log::error!("restoring render context failed: {:?}", e);
        }
    }

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: trait.Widget.html#method.layout
    pub fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        self.inner.layout(layout_ctx, bc, data, &env)
    }

    /// Propagate an event.
    ///
    /// Generally the [`event`] method of a container widget will call this
    /// method on all its children. Here is where a great deal of the event
    /// flow logic resides, particularly whether to continue propagating
    /// the event.
    ///
    /// [`event`]: trait.Widget.html#method.event
    pub fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        // TODO: factor as much logic as possible into monomorphic functions.
        if ctx.is_handled || !event.recurse() {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return;
        }
        let had_active = self.state.has_active;
        let mut child_ctx = EventCtx {
            win_ctx: ctx.win_ctx,
            cursor: ctx.cursor,
            command_queue: ctx.command_queue,
            window: &ctx.window,
            window_id: ctx.window_id,
            widget_id: self.id(),
            base_state: &mut self.state,
            had_active,
            is_handled: false,
            is_root: false,
        };
        let rect = child_ctx.base_state.layout_rect;
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
        let mut hot_changed = None;
        let child_event = match event {
            Event::Size(size) => {
                recurse = ctx.is_root;
                Event::Size(*size)
            }
            Event::MouseDown(mouse_event) => {
                let had_hot = child_ctx.base_state.is_hot;
                let now_hot = rect.winding(mouse_event.pos) != 0;
                if (!had_hot) && now_hot {
                    child_ctx.base_state.is_hot = true;
                    hot_changed = Some(true);
                }
                recurse = had_active || !ctx.had_active && now_hot;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseDown(mouse_event)
            }
            Event::MouseUp(mouse_event) => {
                recurse = had_active || !ctx.had_active && rect.winding(mouse_event.pos) != 0;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseUp(mouse_event)
            }
            Event::MouseMoved(mouse_event) => {
                let had_hot = child_ctx.base_state.is_hot;
                child_ctx.base_state.is_hot = rect.winding(mouse_event.pos) != 0;
                if had_hot != child_ctx.base_state.is_hot {
                    hot_changed = Some(child_ctx.base_state.is_hot);
                }
                recurse = had_active || had_hot || child_ctx.base_state.is_hot;
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos -= rect.origin().to_vec2();
                Event::MouseMoved(mouse_event)
            }
            Event::KeyDown(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::KeyDown(*e)
            }
            Event::KeyUp(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::KeyUp(*e)
            }
            Event::Paste(e) => {
                recurse = child_ctx.base_state.has_focus;
                Event::Paste(e.clone())
            }
            Event::Wheel(wheel_event) => {
                recurse = had_active || child_ctx.base_state.is_hot;
                Event::Wheel(wheel_event.clone())
            }
            Event::Zoom(zoom) => {
                recurse = had_active || child_ctx.base_state.is_hot;
                Event::Zoom(*zoom)
            }
            Event::HotChanged(is_hot) => Event::HotChanged(*is_hot),
            Event::FocusChanged(_is_focused) => {
                let had_focus = child_ctx.base_state.has_focus;
                let focus = child_ctx.base_state.request_focus;
                child_ctx.base_state.request_focus = false;
                child_ctx.base_state.has_focus = focus;
                recurse = focus || had_focus;
                Event::FocusChanged(focus)
            }
            Event::Timer(id) => {
                recurse = child_ctx.base_state.request_timer;
                Event::Timer(*id)
            }
            Event::Command(cmd) => Event::Command(cmd.clone()),
            Event::TargetedCommand(target, cmd) => match target {
                Target::Window(_) => Event::Command(cmd.clone()),
                Target::Widget(id) if *id == self.id => Event::Command(cmd.clone()),
                Target::Widget(id) => {
                    recurse = child_ctx.base_state.children.contains(id);
                    Event::TargetedCommand(*target, cmd.clone())
                }
            },
        };
        child_ctx.base_state.needs_inval = false;
        if let Some(is_hot) = hot_changed {
            let hot_changed_event = Event::HotChanged(is_hot);
            self.inner
                .event(&mut child_ctx, &hot_changed_event, data, &env);
        }
        if recurse {
            child_ctx.base_state.has_active = false;
            self.inner.event(&mut child_ctx, &child_event, data, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
        };
        ctx.base_state.needs_inval |= child_ctx.base_state.needs_inval;
        ctx.base_state.request_anim |= child_ctx.base_state.request_anim;
        ctx.base_state.request_timer |= child_ctx.base_state.request_timer;
        ctx.base_state.is_hot |= child_ctx.base_state.is_hot;
        ctx.base_state.has_active |= child_ctx.base_state.has_active;
        ctx.base_state.request_focus |= child_ctx.base_state.request_focus;
        ctx.base_state.children_changed |= child_ctx.base_state.children_changed;
        ctx.is_handled |= child_ctx.is_handled;
    }

    pub fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        ctx.widget_id = self.id();
        let pre_children = ctx.children;
        let pre_childs_changed = ctx.children_changed;
        let pre_inval = ctx.needs_inval;
        let pre_request_anim = ctx.request_anim;

        ctx.children = Bloom::new();
        ctx.children_changed = false;
        ctx.needs_inval = false;

        let recurse = match event {
            LifeCycle::AnimFrame(_) => {
                let r = self.state.request_anim;
                self.state.request_anim = false;
                ctx.request_anim = false;
                r
            }
            LifeCycle::RegisterChildren => self.state.children_changed,
            _ => true,
        };

        if recurse {
            self.inner.lifecycle(ctx, event, data, env);
            self.state.request_anim = ctx.request_anim;
            ctx.request_anim |= pre_request_anim;
        }

        self.state.children_changed |= ctx.children_changed;
        ctx.children_changed |= pre_childs_changed;
        ctx.needs_inval |= pre_inval;

        if let LifeCycle::RegisterChildren = event {
            self.state.children = ctx.children;
            self.state.children_changed = false;
            ctx.children = ctx.children.intersection(pre_children);
            ctx.register_child(self.id());
        }
    }

    /// Propagate a data update.
    ///
    /// Generally called by container widgets as part of their [`update`]
    /// method.
    ///
    /// [`update`]: trait.Widget.html#method.update
    pub fn update(&mut self, ctx: &mut UpdateCtx, data: &T, env: &Env) {
        let pre_childs_changed = ctx.children_changed;
        let pre_inval = ctx.needs_inval;
        ctx.children_changed = false;
        ctx.needs_inval = false;

        let data_same = if let Some(ref old_data) = self.old_data {
            old_data.same(data)
        } else {
            false
        };
        let env_same = if let Some(ref old_env) = self.env {
            old_env.same(env)
        } else {
            false
        };

        if data_same && env_same {
            return;
        }
        self.inner.update(ctx, self.old_data.as_ref(), data, env);
        self.old_data = Some(data.clone());
        self.env = Some(env.clone());

        self.state.children_changed |= ctx.children_changed;
        ctx.children_changed |= pre_childs_changed;
        ctx.needs_inval |= pre_inval;
    }
}

impl<T: Data, W: Widget<T> + 'static> WidgetPod<T, W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> BoxedWidget<T> {
        WidgetPod::new(Box::new(self.inner))
    }
}

impl BaseState {
    #[inline]
    fn size(&self) -> Size {
        self.layout_rect.size()
    }
}

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
    pub(crate) base_state: &'a BaseState,
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
    /// Query the "hot" state of the widget.
    ///
    /// See [`EventCtx::is_hot`](struct.EventCtx.html#method.is_hot) for
    /// additional information.
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    /// Query the "active" state of the widget.
    ///
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active) for
    /// additional information.
    pub fn is_active(&self) -> bool {
        self.base_state.is_active
    }

    /// Returns the layout size of the current widget.
    ///
    /// See [`EventCtx::size`](struct.EventCtx.html#method.size) for
    /// additional information.
    pub fn size(&self) -> Size {
        self.base_state.size()
    }

    /// Query the focus state of the widget.
    ///
    /// See [`EventCtx::has_focus`](struct.EventCtx.html#method.has_focus) for
    /// additional information.
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

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
            base_state,
            ..
        } = self;
        let mut child_ctx = PaintCtx {
            render_ctx,
            base_state,
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
    pub(crate) command_queue: &'a mut VecDeque<(Target, Command)>,
    pub(crate) window_id: WindowId,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    pub(crate) window: &'a WindowHandle,
    pub(crate) base_state: &'a mut BaseState,
    pub(crate) had_active: bool,
    pub(crate) is_handled: bool,
    pub(crate) is_root: bool,
    pub(crate) widget_id: WidgetId,
}

pub struct LifeCycleCtx<'a> {
    pub(crate) command_queue: &'a mut VecDeque<(Target, Command)>,
    /// the registry for the current widgets children;
    /// only really meaninful during a `LifeCyle::RegisterChildren` call.
    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
    pub(crate) needs_inval: bool,
    pub(crate) request_anim: bool,
    pub(crate) window_id: WindowId,
    pub(crate) widget_id: WidgetId,
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
    pub(crate) command_queue: &'a mut VecDeque<(Target, Command)>,
    // Discussion: we probably want to propagate more fine-grained
    // invalidations, which would mean a structure very much like
    // `EventCtx` (and possibly using the same structure).But for
    // now keep it super-simple.
    pub(crate) needs_inval: bool,
    pub(crate) children_changed: bool,
    pub(crate) window_id: WindowId,
    pub(crate) widget_id: WidgetId,
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

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.base_state.children_changed = true;
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
    /// See [`EventCtx::is_active`](struct.EventCtx.html#method.is_active).
    pub fn set_active(&mut self, active: bool) {
        self.base_state.is_active = active;
        // TODO: plumb mouse grab through to platform (through druid-shell)
    }

    /// The "hot" (aka hover) status of a widget.
    ///
    /// A widget is "hot" when the mouse is hovered over it. Widgets will
    /// often change their appearance as a visual indication that they
    /// will respond to mouse interaction.
    ///
    /// The hot status is computed from the widget's layout rect. In a
    /// container hierarchy, all widgets with layout rects containing the
    /// mouse position have hot status.
    ///
    /// Discussion: there is currently some confusion about whether a
    /// widget can be considered hot when some other widget is active (for
    /// example, when clicking to one widget and dragging to the next).
    /// The documentation should clearly state the resolution.
    pub fn is_hot(&self) -> bool {
        self.base_state.is_hot
    }

    /// The active status of a widget.
    ///
    /// Active status generally corresponds to a mouse button down. Widgets
    /// with behavior similar to a button will call [`set_active`] on mouse
    /// down and then up.
    ///
    /// When a widget is active, it gets mouse events even when the mouse
    /// is dragged away.
    ///
    /// [`set_active`]: struct.EventCtx.html#method.set_active
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

    /// The focus status of a widget.
    ///
    /// Focus means that the widget receives keyboard events.
    ///
    /// A widget can request focus using the [`request_focus`] method.
    /// This will generally result in a separate event propagation of
    /// a `FocusChanged` method, including sending `false` to the previous
    /// widget that held focus.
    ///
    /// Only one leaf widget at a time has focus. However, in a container
    /// hierarchy, all ancestors of that leaf widget are also invoked with
    /// `FocusChanged(true)`.
    ///
    /// Discussion question: is "is_focused" a better name?
    ///
    /// [`request_focus`]: struct.EventCtx.html#method.request_focus
    pub fn has_focus(&self) -> bool {
        self.base_state.has_focus
    }

    /// Request keyboard focus.
    ///
    /// See [`has_focus`] for more information.
    ///
    /// [`has_focus`]: struct.EventCtx.html#method.has_focus
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

    /// The layout size.
    ///
    /// This is the layout size as ultimately determined by the parent
    /// container, on the previous layout pass.
    ///
    /// Generally it will be the same as the size returned by the child widget's
    /// [`layout`] method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
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
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    //pub(crate) fn make_lifecycle_ctx(&mut self) -> LifeCycleCtx {
    //LifeCycleCtx {
    //command_queue: self.command_queue,
    //children_changed: false,
    //needs_inval: false,
    //children: Bloom::default(),
    //request_anim: false,

    //window_id: self.window_id,
    //widget_id: self.widget_id,
    //}
    //}
}

impl<'a> LifeCycleCtx<'a> {
    /// Invalidate.
    ///
    /// See [`EventCtx::invalidate`](struct.EventCtx.html#method.invalidate) for
    /// more discussion.
    pub fn invalidate(&mut self) {
        self.needs_inval = true;
    }

    /// Returns the current widget's `WidgetId`.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Registers a child wildget. This should only be called
    /// in response to a `LifeCycle::RegisterChildren` event.
    pub fn register_child(&mut self, child_id: WidgetId) {
        self.children.add(&child_id);
    }

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.children_changed = true;
    }

    /// Request an animation frame.
    pub fn request_anim_frame(&mut self) {
        self.request_anim = true;
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
        target: impl Into<Option<Target>>,
    ) {
        let target = target.into().unwrap_or_else(|| self.window_id.into());
        self.command_queue.push_back((target, command.into()))
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

    /// Indicate that your children have changed.
    ///
    /// Widgets must call this method after adding a new child.
    pub fn children_changed(&mut self) {
        self.children_changed = true;
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
    //TODO: can we delete this? where is it used?
    pub fn window(&self) -> &WindowHandle {
        &self.window
    }

    /// Get the window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// get the `WidgetId` of the current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub(crate) fn make_lifecycle_ctx(&mut self) -> LifeCycleCtx {
        LifeCycleCtx {
            command_queue: self.command_queue,
            children: Bloom::default(),
            children_changed: false,
            needs_inval: false,
            request_anim: false,
            window_id: self.window_id,
            widget_id: self.widget_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Flex, IdentityWrapper, Scroll, Split, TextBox, WidgetExt};

    #[test]
    fn register_children() {
        fn make_widgets() -> (WidgetId, WidgetId, WidgetId, impl Widget<Option<u32>>) {
            let (id1, t1) = IdentityWrapper::wrap(TextBox::raw().parse());
            let (id2, t2) = IdentityWrapper::wrap(TextBox::raw().parse());
            let (id3, t3) = IdentityWrapper::wrap(TextBox::raw().parse());
            eprintln!("{:?}, {:?}, {:?}", id1, id2, id3);
            let widget = Split::vertical(
                Flex::row()
                    .with_child(t1, 1.0)
                    .with_child(t2, 1.0)
                    .with_child(t3, 1.0),
                Scroll::new(TextBox::raw().parse()),
            );
            (id1, id2, id3, widget)
        }

        let (id1, id2, id3, widget) = make_widgets();
        let mut widget = WidgetPod::new(widget).boxed();

        let mut command_queue: VecDeque<(Target, Command)> = VecDeque::new();
        let mut ctx = LifeCycleCtx {
            command_queue: &mut command_queue,
            children: Bloom::new(),
            children_changed: true,
            needs_inval: false,
            request_anim: false,
            window_id: WindowId::next(),
            widget_id: WidgetId::next(),
        };

        let env = Env::default();

        widget.lifecycle(&mut ctx, &LifeCycle::RegisterChildren, &None, &env);
        assert!(ctx.children.contains(&id1));
        assert!(ctx.children.contains(&id2));
        assert!(ctx.children.contains(&id3));
        assert_eq!(ctx.children.entry_count(), 7);
    }
}
