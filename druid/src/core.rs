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

//! The fundamental druid types.

use std::collections::{HashMap, VecDeque};

use crate::bloom::Bloom;
use crate::contexts::ContextState;
use crate::kurbo::{Affine, Insets, Point, Rect, Shape, Size, Vec2};
use crate::util::ExtendDrain;
use crate::{
    ArcStr, BoxConstraints, Color, Command, Cursor, Data, Env, Event, EventCtx, InternalEvent,
    InternalLifeCycle, LayoutCtx, LifeCycle, LifeCycleCtx, Notification, PaintCtx, Region,
    RenderContext, Target, TextLayout, TimerToken, UpdateCtx, Widget, WidgetId,
};

/// Our queue type
pub(crate) type CommandQueue = VecDeque<Command>;

/// A container for one widget in the hierarchy.
///
/// Generally, container widgets don't contain other widgets directly,
/// but rather contain a `WidgetPod`, which has additional state needed
/// for layout and for the widget to participate in event flow.
///
/// `WidgetPod` will translate internal druid events to regular events,
/// synthesize additional events of interest, and stop propagation when it makes sense.
///
/// This struct also contains the previous data for a widget, which is
/// essential for the [`update`] method, both to decide when the update
/// needs to propagate, and to provide the previous data so that a
/// widget can process a diff between the old value and the new.
///
/// [`update`]: trait.Widget.html#tymethod.update
pub struct WidgetPod<T, W> {
    state: WidgetState,
    old_data: Option<T>,
    env: Option<Env>,
    inner: W,
    // stashed layout so we don't recompute this when debugging
    debug_widget_text: TextLayout<ArcStr>,
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
#[derive(Clone)]
pub(crate) struct WidgetState {
    pub(crate) id: WidgetId,
    /// The size of the child; this is the value returned by the child's layout
    /// method.
    size: Size,
    /// The origin of the child in the parent's coordinate space; together with
    /// `size` these constitute the child's layout rect.
    origin: Point,
    /// A flag used to track and debug missing calls to set_origin.
    is_expecting_set_origin_call: bool,
    /// The insets applied to the layout rect to generate the paint rect.
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    pub(crate) paint_insets: Insets,

    /// The offset of the baseline relative to the bottom of the widget.
    ///
    /// In general, this will be zero; the bottom of the widget will be considered
    /// the baseline. Widgets that contain text or controls that expect to be
    /// laid out alongside text can set this as appropriate.
    pub(crate) baseline_offset: f64,

    // The region that needs to be repainted, relative to the widget's bounds.
    pub(crate) invalid: Region,

    // The part of this widget that is visible on the screen is offset by this
    // much. This will be non-zero for widgets that are children of `Scroll`, or
    // similar, and it is used for propagating invalid regions.
    pub(crate) viewport_offset: Vec2,

    // TODO: consider using bitflags for the booleans.
    pub(crate) is_hot: bool,

    pub(crate) is_active: bool,

    pub(crate) needs_layout: bool,

    /// Any descendant is active.
    has_active: bool,

    /// In the focused path, starting from window and ending at the focused widget.
    /// Descendants of the focused widget are not in the focused path.
    pub(crate) has_focus: bool,

    /// Any descendant has requested an animation frame.
    pub(crate) request_anim: bool,

    /// Any descendant has requested update.
    pub(crate) request_update: bool,

    pub(crate) focus_chain: Vec<WidgetId>,
    pub(crate) request_focus: Option<FocusChange>,
    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
    /// Associate timers with widgets that requested them.
    pub(crate) timers: HashMap<TimerToken, WidgetId>,
    /// The cursor that was set using one of the context methods.
    pub(crate) cursor_change: CursorChange,
    /// The result of merging up children cursors. This gets cleared when merging state up (unlike
    /// cursor_change, which is persistent).
    pub(crate) cursor: Option<Cursor>,
}

/// Methods by which a widget can attempt to change focus state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FocusChange {
    /// The focused widget is giving up focus.
    Resign,
    /// A specific widget wants focus
    Focus(WidgetId),
    /// Focus should pass to the next focusable widget
    Next,
    /// Focus should pass to the previous focusable widget
    Previous,
}

/// The possible cursor states for a widget.
#[derive(Clone)]
pub(crate) enum CursorChange {
    /// No cursor has been set.
    Default,
    /// Someone set a cursor, but if a child widget also set their cursor then we'll use theirs
    /// instead of ours.
    Set(Cursor),
    /// Someone set a cursor, and we'll use it regardless of what the children say.
    Override(Cursor),
}

impl<T, W: Widget<T>> WidgetPod<T, W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<T, W> {
        let mut state = WidgetState::new(inner.id().unwrap_or_else(WidgetId::next), None);
        state.children_changed = true;
        state.needs_layout = true;
        WidgetPod {
            state,
            old_data: None,
            env: None,
            inner,
            debug_widget_text: TextLayout::new(),
        }
    }

    /// Read-only access to state. We don't mark the field as `pub` because
    /// we want to control mutation.
    pub(crate) fn state(&self) -> &WidgetState {
        &self.state
    }

    /// Returns `true` if the widget has received [`LifeCycle::WidgetAdded`].
    ///
    /// [`LifeCycle::WidgetAdded`]: ./enum.LifeCycle.html#variant.WidgetAdded
    pub fn is_initialized(&self) -> bool {
        self.old_data.is_some()
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
    ///
    /// See [`EventCtx::is_hot`](struct.EventCtx.html#method.is_hot) for
    /// additional information.
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
        self.state.id
    }

    /// Set the layout [`Rect`].
    ///
    /// This is soft-deprecated; you should use [`set_origin`] instead for new code.
    ///
    /// [`set_origin`]: WidgetPod::set_origin
    pub fn set_layout_rect(&mut self, ctx: &mut LayoutCtx, data: &T, env: &Env, layout_rect: Rect) {
        if layout_rect.size() != self.state.size {
            log::warn!("set_layout_rect passed different size than returned by layout method");
        }
        self.set_origin(ctx, data, env, layout_rect.origin());
    }

    /// Set the origin of this widget, in the parent's coordinate space.
    ///
    /// A container widget should call the [`Widget::layout`] method on its children in
    /// its own [`Widget::layout`] implementation, and then call `set_origin` to
    /// position those children.
    ///
    /// The child will receive the [`LifeCycle::Size`] event informing them of the final [`Size`].
    ///
    /// [`Widget::layout`]: trait.Widget.html#tymethod.layout
    /// [`Rect`]: struct.Rect.html
    /// [`Size`]: struct.Size.html
    /// [`LifeCycle::Size`]: enum.LifeCycle.html#variant.Size
    pub fn set_origin(&mut self, ctx: &mut LayoutCtx, data: &T, env: &Env, origin: Point) {
        self.state.origin = origin;
        self.state.is_expecting_set_origin_call = false;
        let layout_rect = self.layout_rect();

        // if the widget has moved, it may have moved under the mouse, in which
        // case we need to handle that.
        if WidgetPod::set_hot_state(
            &mut self.inner,
            &mut self.state,
            ctx.state,
            layout_rect,
            ctx.mouse_pos,
            data,
            env,
        ) {
            ctx.widget_state.merge_up(&mut self.state);
        }
    }

    /// Returns the layout [`Rect`].
    ///
    /// This will be a [`Rect`] with a [`Size`] determined by the child's [`layout`]
    /// method, and the origin that was set by [`set_origin`].
    ///
    /// [`Rect`]: struct.Rect.html
    /// [`Size`]: struct.Size.html
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`set_origin`]: WidgetPod::set_origin
    pub fn layout_rect(&self) -> Rect {
        self.state.layout_rect()
    }

    /// Set the viewport offset.
    ///
    /// This is relevant only for children of a scroll view (or similar). It must
    /// be set by the parent widget whenever it modifies the position of its child
    /// while painting it and propagating events. As a rule of thumb, you need this
    /// if and only if you `Affine::translate` the paint context before painting
    /// your child. For an example, see the implentation of [`Scroll`].
    ///
    /// [`Scroll`]: widget/struct.Scroll.html
    pub fn set_viewport_offset(&mut self, offset: Vec2) {
        self.state.viewport_offset = offset;
    }

    /// The viewport offset.
    ///
    /// This will be the same value as set by [`set_viewport_offset`].
    ///
    /// [`set_viewport_offset`]: #method.viewport_offset
    pub fn viewport_offset(&self) -> Vec2 {
        self.state.viewport_offset
    }

    /// Get the widget's paint [`Rect`].
    ///
    /// This is the [`Rect`] that widget has indicated it needs to paint in.
    /// This is the same as the [`layout_rect`] with the [`paint_insets`] applied;
    /// in the general case it is the same as the [`layout_rect`].
    ///
    /// [`layout_rect`]: #method.layout_rect
    /// [`Rect`]: struct.Rect.html
    /// [`paint_insets`]: #method.paint_insets
    pub fn paint_rect(&self) -> Rect {
        self.state.paint_rect()
    }

    /// Return the paint [`Insets`] for this widget.
    ///
    /// If these [`Insets`] are nonzero, they describe the area beyond a widget's
    /// layout rect where it needs to paint.
    ///
    /// These are generally zero; exceptions are widgets that do things like
    /// paint a drop shadow.
    ///
    /// A widget can set its insets by calling [`set_paint_insets`] during its
    /// [`layout`] method.
    ///
    /// [`Insets`]: struct.Insets.html
    /// [`set_paint_insets`]: struct.LayoutCtx.html#method.set_paint_insets
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn paint_insets(&self) -> Insets {
        self.state.paint_insets
    }

    /// Given a parents layout size, determine the appropriate paint `Insets`
    /// for the parent.
    ///
    /// This is a convenience method to be used from the [`layout`] method
    /// of a `Widget` that manages a child; it allows the parent to correctly
    /// propogate a child's desired paint rect, if it extends beyond the bounds
    /// of the parent's layout rect.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`Insets`]: struct.Insets.html
    pub fn compute_parent_paint_insets(&self, parent_size: Size) -> Insets {
        let parent_bounds = Rect::ZERO.with_size(parent_size);
        let union_pant_rect = self.paint_rect().union(parent_bounds);
        union_pant_rect - parent_bounds
    }

    /// The distance from the bottom of this widget to the baseline.
    pub fn baseline_offset(&self) -> f64 {
        self.state.baseline_offset
    }

    /// Determines if the provided `mouse_pos` is inside `rect`
    /// and if so updates the hot state and sends `LifeCycle::HotChanged`.
    ///
    /// Returns `true` if the hot state changed.
    ///
    /// The provided `child_state` should be merged up if this returns `true`.
    fn set_hot_state(
        child: &mut W,
        child_state: &mut WidgetState,
        state: &mut ContextState,
        rect: Rect,
        mouse_pos: Option<Point>,
        data: &T,
        env: &Env,
    ) -> bool {
        let had_hot = child_state.is_hot;
        child_state.is_hot = match mouse_pos {
            Some(pos) => rect.winding(pos) != 0,
            None => false,
        };
        if had_hot != child_state.is_hot {
            let hot_changed_event = LifeCycle::HotChanged(child_state.is_hot);
            let mut child_ctx = LifeCycleCtx {
                state,
                widget_state: child_state,
            };
            child.lifecycle(&mut child_ctx, &hot_changed_event, data, env);
            // if hot changes and we're showing widget ids, always repaint
            if env.get(Env::DEBUG_WIDGET_ID) {
                child_ctx.request_paint();
            }
            return true;
        }
        false
    }
}

impl<T: Data, W: Widget<T>> WidgetPod<T, W> {
    /// Paint a child widget.
    ///
    /// Generally called by container widgets as part of their [`Widget::paint`]
    /// method.
    ///
    /// Note that this method does not apply the offset of the layout rect.
    /// If that is desired, use [`paint`] instead.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    /// [`Widget::paint`]: trait.Widget.html#tymethod.paint
    /// [`paint`]: #method.paint
    pub fn paint_raw(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        // we need to do this before we borrow from self
        if env.get(Env::DEBUG_WIDGET_ID) {
            self.make_widget_id_layout_if_needed(self.state.id, ctx, env);
        }

        let mut inner_ctx = PaintCtx {
            render_ctx: ctx.render_ctx,
            state: ctx.state,
            z_ops: Vec::new(),
            region: ctx.region.clone(),
            widget_state: &self.state,
            depth: ctx.depth,
        };
        self.inner.paint(&mut inner_ctx, data, env);

        let debug_ids = inner_ctx.is_hot() && env.get(Env::DEBUG_WIDGET_ID);
        if debug_ids {
            // this also draws layout bounds
            self.debug_paint_widget_ids(&mut inner_ctx, env);
        }

        if !debug_ids && env.get(Env::DEBUG_PAINT) {
            self.debug_paint_layout_bounds(&mut inner_ctx, env);
        }

        ctx.z_ops.append(&mut inner_ctx.z_ops);
    }

    /// Paint the widget, translating it by the origin of its layout rectangle.
    ///
    /// This will recursively paint widgets, stopping if a widget's layout
    /// rect is outside of the currently visible region.
    pub fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_impl(ctx, data, env, false)
    }

    /// Paint the widget, even if its layout rect is outside of the currently
    /// visible region.
    pub fn paint_always(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.paint_impl(ctx, data, env, true)
    }

    /// Shared implementation that can skip drawing non-visible content.
    fn paint_impl(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env, paint_if_not_visible: bool) {
        if !paint_if_not_visible && !ctx.region().intersects(self.state.paint_rect()) {
            return;
        }

        if !self.is_initialized() {
            debug_panic!(
                "{:?}: paint method called before receiving WidgetAdded.",
                ctx.widget_id()
            );
            return;
        }

        ctx.with_save(|ctx| {
            let layout_origin = self.layout_rect().origin().to_vec2();
            ctx.transform(Affine::translate(layout_origin));
            let mut visible = ctx.region().clone();
            visible.intersect_with(self.state.paint_rect());
            visible -= layout_origin;
            ctx.with_child_ctx(visible, |ctx| self.paint_raw(ctx, data, env));
        });
    }

    fn make_widget_id_layout_if_needed(&mut self, id: WidgetId, ctx: &mut PaintCtx, env: &Env) {
        if self.debug_widget_text.needs_rebuild() {
            // switch text color based on background, this is meh and that's okay
            let border_color = env.get_debug_color(id.to_raw());
            let (r, g, b, _) = border_color.as_rgba8();
            let avg = (r as u32 + g as u32 + b as u32) / 3;
            let text_color = if avg < 128 {
                Color::WHITE
            } else {
                Color::BLACK
            };
            let id_string = id.to_raw().to_string();
            self.debug_widget_text.set_text(id_string.into());
            self.debug_widget_text.set_text_size(10.0);
            self.debug_widget_text.set_text_color(text_color);
            self.debug_widget_text.rebuild_if_needed(ctx.text(), env);
        }
    }

    fn debug_paint_widget_ids(&self, ctx: &mut PaintCtx, env: &Env) {
        // we clone because we need to move it for paint_with_z_index
        let text = self.debug_widget_text.clone();
        let text_size = text.size();
        let origin = ctx.size().to_vec2() - text_size.to_vec2();
        let border_color = env.get_debug_color(ctx.widget_id().to_raw());
        self.debug_paint_layout_bounds(ctx, env);

        ctx.paint_with_z_index(ctx.depth(), move |ctx| {
            let origin = Point::new(origin.x.max(0.0), origin.y.max(0.0));
            let text_rect = Rect::from_origin_size(origin, text_size);
            ctx.fill(text_rect, &border_color);
            text.draw(ctx, origin);
        })
    }

    fn debug_paint_layout_bounds(&self, ctx: &mut PaintCtx, env: &Env) {
        const BORDER_WIDTH: f64 = 1.0;
        let rect = ctx.size().to_rect().inset(BORDER_WIDTH / -2.0);
        let id = self.id().to_raw();
        let color = env.get_debug_color(id);
        ctx.stroke(rect, &color, BORDER_WIDTH);
    }

    /// Compute layout of a widget.
    ///
    /// Generally called by container widgets as part of their [`layout`]
    /// method.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    pub fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        if !self.is_initialized() {
            debug_panic!(
                "{:?}: layout method called before receiving WidgetAdded.",
                ctx.widget_id()
            );
            return Size::ZERO;
        }

        self.state.needs_layout = false;
        self.state.is_expecting_set_origin_call = true;

        let child_mouse_pos = match ctx.mouse_pos {
            Some(pos) => Some(pos - self.layout_rect().origin().to_vec2()),
            None => None,
        };
        let prev_size = self.state.size;

        let mut child_ctx = LayoutCtx {
            widget_state: &mut self.state,
            state: ctx.state,
            mouse_pos: child_mouse_pos,
        };

        let new_size = self.inner.layout(&mut child_ctx, bc, data, env);
        if new_size != prev_size {
            let mut child_ctx = LifeCycleCtx {
                widget_state: child_ctx.widget_state,
                state: child_ctx.state,
            };
            let size_event = LifeCycle::Size(new_size);
            self.inner.lifecycle(&mut child_ctx, &size_event, data, env);
        }

        ctx.widget_state.merge_up(&mut child_ctx.widget_state);
        self.state.size = new_size;
        self.log_layout_issues(new_size);

        new_size
    }

    fn log_layout_issues(&self, size: Size) {
        if size.width.is_infinite() {
            let name = self.widget().type_name();
            log::warn!("Widget `{}` has an infinite width.", name);
        }
        if size.height.is_infinite() {
            let name = self.widget().type_name();
            log::warn!("Widget `{}` has an infinite height.", name);
        }
    }

    /// Execute the closure with this widgets `EventCtx`.
    #[cfg(feature = "crochet")]
    pub fn with_event_context<F>(&mut self, parent_ctx: &mut EventCtx, mut fun: F)
    where
        F: FnMut(&mut W, &mut EventCtx),
    {
        let mut ctx = EventCtx {
            state: parent_ctx.state,
            widget_state: &mut self.state,
            notifications: parent_ctx.notifications,
            is_handled: false,
            is_root: false,
        };
        fun(&mut self.inner, &mut ctx);
        parent_ctx.widget_state.merge_up(&mut self.state);
    }

    /// Propagate an event.
    ///
    /// Generally the [`event`] method of a container widget will call this
    /// method on all its children. Here is where a great deal of the event
    /// flow logic resides, particularly whether to continue propagating
    /// the event.
    ///
    /// [`event`]: trait.Widget.html#tymethod.event
    pub fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if !self.is_initialized() {
            debug_panic!(
                "{:?}: event method called before receiving WidgetAdded.",
                ctx.widget_id()
            );
            return;
        }

        // log if we seem not to be laid out when we should be
        if self.state.is_expecting_set_origin_call && !event.should_propagate_to_hidden() {
            log::warn!(
                "{:?} received an event ({:?}) without having been laid out. \
                This likely indicates a missed call to set_layout_rect.",
                ctx.widget_id(),
                event,
            );
        }

        // TODO: factor as much logic as possible into monomorphic functions.
        if ctx.is_handled {
            // This function is called by containers to propagate an event from
            // containers to children. Non-recurse events will be invoked directly
            // from other points in the library.
            return;
        }
        let had_active = self.state.has_active;
        let rect = self.layout_rect();

        // If we need to replace either the event or its data.
        let mut modified_event = None;

        let recurse = match event {
            Event::Internal(internal) => match internal {
                InternalEvent::MouseLeave => {
                    let hot_changed = WidgetPod::set_hot_state(
                        &mut self.inner,
                        &mut self.state,
                        ctx.state,
                        rect,
                        None,
                        data,
                        env,
                    );
                    had_active || hot_changed
                }
                InternalEvent::TargetedCommand(cmd) => {
                    match cmd.target() {
                        Target::Widget(id) if id == self.id() => {
                            modified_event = Some(Event::Command(cmd.clone()));
                            true
                        }
                        Target::Widget(id) => {
                            // Recurse when the target widget could be our descendant.
                            // The bloom filter we're checking can return false positives.
                            self.state.children.may_contain(&id)
                        }
                        Target::Global | Target::Window(_) => {
                            modified_event = Some(Event::Command(cmd.clone()));
                            true
                        }
                        _ => false,
                    }
                }
                InternalEvent::RouteTimer(token, widget_id) => {
                    if *widget_id == self.id() {
                        modified_event = Some(Event::Timer(*token));
                        true
                    } else {
                        self.state.children.may_contain(widget_id)
                    }
                }
            },
            Event::WindowConnected => true,
            Event::WindowSize(_) => {
                self.state.needs_layout = true;
                ctx.is_root
            }
            Event::MouseDown(mouse_event) => {
                WidgetPod::set_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    ctx.state,
                    rect,
                    Some(mouse_event.pos),
                    data,
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseDown(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseUp(mouse_event) => {
                WidgetPod::set_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    ctx.state,
                    rect,
                    Some(mouse_event.pos),
                    data,
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseUp(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::MouseMove(mouse_event) => {
                let hot_changed = WidgetPod::set_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    ctx.state,
                    rect,
                    Some(mouse_event.pos),
                    data,
                    env,
                );
                // MouseMove is recursed even if the widget is not active and not hot,
                // but was hot previously. This is to allow the widget to respond to the movement,
                // e.g. drag functionality where the widget wants to follow the mouse.
                if had_active || self.state.is_hot || hot_changed {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::MouseMove(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::Wheel(mouse_event) => {
                WidgetPod::set_hot_state(
                    &mut self.inner,
                    &mut self.state,
                    ctx.state,
                    rect,
                    Some(mouse_event.pos),
                    data,
                    env,
                );
                if had_active || self.state.is_hot {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos -= rect.origin().to_vec2();
                    modified_event = Some(Event::Wheel(mouse_event));
                    true
                } else {
                    false
                }
            }
            Event::AnimFrame(_) => {
                let r = self.state.request_anim;
                self.state.request_anim = false;
                r
            }
            Event::KeyDown(_) => self.state.has_focus,
            Event::KeyUp(_) => self.state.has_focus,
            Event::Paste(_) => self.state.has_focus,
            Event::Zoom(_) => had_active || self.state.is_hot,
            Event::Timer(_) => false, // This event was targeted only to our parent
            Event::Command(_) => true,
            Event::Notification(_) => false,
        };

        if recurse {
            let mut notifications = VecDeque::new();
            let mut inner_ctx = EventCtx {
                state: ctx.state,
                widget_state: &mut self.state,
                notifications: &mut notifications,
                is_handled: false,
                is_root: false,
            };
            let inner_event = modified_event.as_ref().unwrap_or(event);
            inner_ctx.widget_state.has_active = false;

            self.inner.event(&mut inner_ctx, &inner_event, data, env);

            inner_ctx.widget_state.has_active |= inner_ctx.widget_state.is_active;
            ctx.is_handled |= inner_ctx.is_handled;
            // we try to handle the notifications that occured below us in the tree
            self.send_notifications(ctx, &mut notifications, data, env);
        }

        // Always merge even if not needed, because merging is idempotent and gives us simpler code.
        // Doing this conditionally only makes sense when there's a measurable performance boost.
        ctx.widget_state.merge_up(&mut self.state);
    }

    /// Send notifications originating from this widget's children to this
    /// widget.
    ///
    /// Notifications that are unhandled will be added to the notification
    /// list for the parent's `EventCtx`, to be retried there.
    fn send_notifications(
        &mut self,
        ctx: &mut EventCtx,
        notifications: &mut VecDeque<Notification>,
        data: &mut T,
        env: &Env,
    ) {
        let EventCtx {
            state,
            notifications: parent_notifications,
            ..
        } = ctx;
        let mut sentinal = VecDeque::new();
        let mut inner_ctx = EventCtx {
            state,
            notifications: &mut sentinal,
            widget_state: &mut self.state,
            is_handled: false,
            is_root: false,
        };

        for _ in 0..notifications.len() {
            let notification = notifications.pop_front().unwrap();
            let event = Event::Notification(notification);
            self.inner.event(&mut inner_ctx, &event, data, env);
            if inner_ctx.is_handled {
                inner_ctx.is_handled = false;
            } else if let Event::Notification(notification) = event {
                // we will try again with the next parent
                parent_notifications.push_back(notification);
            } else {
                unreachable!()
            }
        }

        if !inner_ctx.notifications.is_empty() {
            log::warn!(
                "A Notification was submitted while handling another \
            notification; the submitted notification will be ignored."
            );
        }
    }

    /// Propagate a [`LifeCycle`] event.
    ///
    /// [`LifeCycle`]: enum.LifeCycle.html
    pub fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        // in the case of an internal routing event, if we are at our target
        // we may send an extra event after the actual event
        let mut extra_event = None;

        let recurse = match event {
            LifeCycle::Internal(internal) => match internal {
                InternalLifeCycle::RouteWidgetAdded => {
                    // if this is called either we were just created, in
                    // which case we need to change lifecycle event to
                    // WidgetAdded or in case we were already created
                    // we just pass this event down
                    if self.old_data.is_none() {
                        self.lifecycle(ctx, &LifeCycle::WidgetAdded, data, env);
                        return;
                    } else {
                        if self.state.children_changed {
                            self.state.children.clear();
                            self.state.focus_chain.clear();
                        }
                        self.state.children_changed
                    }
                }
                InternalLifeCycle::RouteFocusChanged { old, new } => {
                    let this_changed = if *old == Some(self.state.id) {
                        Some(false)
                    } else if *new == Some(self.state.id) {
                        Some(true)
                    } else {
                        None
                    };

                    if let Some(change) = this_changed {
                        self.state.has_focus = change;
                        extra_event = Some(LifeCycle::FocusChanged(change));
                    } else {
                        self.state.has_focus = false;
                    }

                    // Recurse when the target widgets could be our descendants.
                    // The bloom filter we're checking can return false positives.
                    match (old, new) {
                        (Some(old), _) if self.state.children.may_contain(old) => true,
                        (_, Some(new)) if self.state.children.may_contain(new) => true,
                        _ => false,
                    }
                }
                #[cfg(test)]
                InternalLifeCycle::DebugRequestState { widget, state_cell } => {
                    if *widget == self.id() {
                        state_cell.set(self.state.clone());
                        false
                    } else {
                        // Recurse when the target widget could be our descendant.
                        // The bloom filter we're checking can return false positives.
                        self.state.children.may_contain(&widget)
                    }
                }
                #[cfg(test)]
                InternalLifeCycle::DebugInspectState(f) => {
                    f.call(&self.state);
                    true
                }
            },
            LifeCycle::WidgetAdded => {
                assert!(self.old_data.is_none());

                self.old_data = Some(data.clone());
                self.env = Some(env.clone());

                true
            }
            _ if !self.is_initialized() => {
                debug_panic!(
                    "{:?}: received LifeCycle::{:?} before WidgetAdded.",
                    self.id(),
                    event
                );
                return;
            }
            LifeCycle::Size(_) => {
                // We are a descendant of a widget that received the Size event.
                // This event was meant only for our parent, so don't recurse.
                false
            }
            //NOTE: this is not sent here, but from the special set_hot_state method
            LifeCycle::HotChanged(_) => false,
            LifeCycle::FocusChanged(_) => {
                // We are a descendant of a widget that has/had focus.
                // Descendants don't inherit focus, so don't recurse.
                false
            }
        };

        let mut child_ctx = LifeCycleCtx {
            state: ctx.state,
            widget_state: &mut self.state,
        };

        if recurse {
            self.inner.lifecycle(&mut child_ctx, event, data, env);
        }

        if let Some(event) = extra_event.as_ref() {
            self.inner.lifecycle(&mut child_ctx, event, data, env);
        }

        ctx.widget_state.merge_up(&mut self.state);

        // we need to (re)register children in case of one of the following events
        match event {
            LifeCycle::WidgetAdded | LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded) => {
                self.state.children_changed = false;
                ctx.widget_state.children = ctx.widget_state.children.union(self.state.children);
                ctx.widget_state.focus_chain.extend(&self.state.focus_chain);
                ctx.register_child(self.id());
            }
            _ => (),
        }
    }

    /// Propagate a data update.
    ///
    /// Generally called by container widgets as part of their [`update`]
    /// method.
    ///
    /// [`update`]: trait.Widget.html#tymethod.update
    pub fn update(&mut self, ctx: &mut UpdateCtx, data: &T, env: &Env) {
        if !self.state.request_update {
            match (self.old_data.as_ref(), self.env.as_ref()) {
                (Some(d), Some(e)) if d.same(data) && e.same(env) => return,
                (Some(_), None) => self.env = Some(env.clone()),
                (None, _) => {
                    debug_panic!(
                        "{:?} is receiving an update without having first received WidgetAdded.",
                        self.id()
                    );
                    return;
                }
                (Some(_), Some(_)) => {}
            }
        }

        let prev_env = self.env.as_ref().filter(|p| !p.same(env));

        let mut child_ctx = UpdateCtx {
            state: ctx.state,
            widget_state: &mut self.state,
            prev_env,
            env,
        };

        self.inner
            .update(&mut child_ctx, self.old_data.as_ref().unwrap(), data, env);
        self.old_data = Some(data.clone());
        self.env = Some(env.clone());

        self.state.request_update = false;
        ctx.widget_state.merge_up(&mut self.state);
    }
}

impl<T, W: Widget<T> + 'static> WidgetPod<T, W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> WidgetPod<T, Box<dyn Widget<T>>> {
        WidgetPod::new(Box::new(self.inner))
    }
}

impl WidgetState {
    pub(crate) fn new(id: WidgetId, size: Option<Size>) -> WidgetState {
        WidgetState {
            id,
            origin: Point::ORIGIN,
            size: size.unwrap_or_default(),
            is_expecting_set_origin_call: true,
            paint_insets: Insets::ZERO,
            invalid: Region::EMPTY,
            viewport_offset: Vec2::ZERO,
            baseline_offset: 0.0,
            is_hot: false,
            needs_layout: false,
            is_active: false,
            has_active: false,
            has_focus: false,
            request_anim: false,
            request_update: false,
            request_focus: None,
            focus_chain: Vec::new(),
            children: Bloom::new(),
            children_changed: false,
            timers: HashMap::new(),
            cursor_change: CursorChange::Default,
            cursor: None,
        }
    }

    pub(crate) fn add_timer(&mut self, timer_token: TimerToken) {
        self.timers.insert(timer_token, self.id);
    }

    /// Update to incorporate state changes from a child.
    ///
    /// This will also clear some requests in the child state.
    ///
    /// This method is idempotent and can be called multiple times.
    fn merge_up(&mut self, child_state: &mut WidgetState) {
        let clip = self
            .layout_rect()
            .with_origin(Point::ORIGIN)
            .inset(self.paint_insets);
        let offset = child_state.layout_rect().origin().to_vec2() - child_state.viewport_offset;
        for &r in child_state.invalid.rects() {
            let r = (r + offset).intersect(clip);
            if r.area() != 0.0 {
                self.invalid.add_rect(r);
            }
        }
        // Clearing the invalid rects here is less fragile than doing it while painting. The
        // problem is that widgets (for example, Either) might choose not to paint certain
        // invisible children, and we shouldn't allow these invisible children to accumulate
        // invalid rects.
        child_state.invalid.clear();

        self.needs_layout |= child_state.needs_layout;
        self.request_anim |= child_state.request_anim;
        self.has_active |= child_state.has_active;
        self.has_focus |= child_state.has_focus;
        self.children_changed |= child_state.children_changed;
        self.request_update |= child_state.request_update;
        self.request_focus = child_state.request_focus.take().or(self.request_focus);
        self.timers.extend_drain(&mut child_state.timers);

        // We reset `child_state.cursor` no matter what, so that on the every pass through the tree,
        // things will be recalculated just from `cursor_change`.
        let child_cursor = child_state.cursor.take();
        if let CursorChange::Override(cursor) = &self.cursor_change {
            self.cursor = Some(cursor.clone());
        } else if child_state.has_active || child_state.is_hot {
            self.cursor = child_cursor;
        }

        if self.cursor.is_none() {
            if let CursorChange::Set(cursor) = &self.cursor_change {
                self.cursor = Some(cursor.clone());
            }
        }
    }

    #[inline]
    pub(crate) fn size(&self) -> Size {
        self.size
    }

    /// The paint region for this widget.
    ///
    /// For more information, see [`WidgetPod::paint_rect`].
    ///
    /// [`WidgetPod::paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub(crate) fn paint_rect(&self) -> Rect {
        self.layout_rect() + self.paint_insets
    }

    pub(crate) fn layout_rect(&self) -> Rect {
        Rect::from_origin_size(self.origin, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ext_event::ExtEventHost;
    use crate::widget::{Flex, Scroll, Split, TextBox};
    use crate::{WidgetExt, WindowHandle, WindowId};

    const ID_1: WidgetId = WidgetId::reserved(0);
    const ID_2: WidgetId = WidgetId::reserved(1);
    const ID_3: WidgetId = WidgetId::reserved(2);

    #[test]
    fn register_children() {
        fn make_widgets() -> impl Widget<Option<u32>> {
            Split::columns(
                Flex::<Option<u32>>::row()
                    .with_child(TextBox::new().with_id(ID_1).parse())
                    .with_child(TextBox::new().with_id(ID_2).parse())
                    .with_child(TextBox::new().with_id(ID_3).parse()),
                Scroll::new(TextBox::new().parse()),
            )
        }

        let widget = make_widgets();
        let mut widget = WidgetPod::new(widget).boxed();

        let mut command_queue: CommandQueue = VecDeque::new();
        let mut widget_state = WidgetState::new(WidgetId::next(), None);
        let window = WindowHandle::default();
        let ext_host = ExtEventHost::default();
        let ext_handle = ext_host.make_sink();
        let mut state = ContextState::new::<Option<u32>>(
            &mut command_queue,
            &ext_handle,
            &window,
            WindowId::next(),
            None,
        );

        let mut ctx = LifeCycleCtx {
            widget_state: &mut widget_state,
            state: &mut state,
        };

        let env = Env::default();

        widget.lifecycle(&mut ctx, &LifeCycle::WidgetAdded, &None, &env);
        assert!(ctx.widget_state.children.may_contain(&ID_1));
        assert!(ctx.widget_state.children.may_contain(&ID_2));
        assert!(ctx.widget_state.children.may_contain(&ID_3));
        assert_eq!(ctx.widget_state.children.entry_count(), 7);
    }
}
