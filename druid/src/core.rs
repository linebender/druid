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

use log;

use crate::bloom::Bloom;
use crate::kurbo::{Affine, Insets, Point, Rect, Shape, Size};
use crate::piet::RenderContext;
use crate::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Target, UpdateCtx, Widget, WidgetId,
};

/// Convenience type for dynamic boxed widget.
pub type BoxedWidget<T> = WidgetPod<T, Box<dyn Widget<T>>>;

/// Our queue type
pub(crate) type CommandQueue = VecDeque<(Target, Command)>;

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
/// [`update`]: widget/trait.Widget.html#tymethod.update
pub struct WidgetPod<T, W> {
    state: BaseState,
    old_data: Option<T>,
    env: Option<Env>,
    inner: W,
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
/// [`paint`]: widget/trait.Widget.html#tymethod.paint
/// [`WidgetPod`]: struct.WidgetPod.html
#[derive(Clone)]
pub(crate) struct BaseState {
    pub(crate) id: WidgetId,
    pub(crate) layout_rect: Rect,
    /// The insets applied to the layout rect to generate the paint rect.
    /// In general, these will be zero; the exception is for things like
    /// drop shadows or overflowing text.
    paint_insets: Insets,

    // TODO: consider using bitflags for the booleans.

    // This should become an invalidation rect.
    pub(crate) needs_inval: bool,

    pub(crate) is_hot: bool,

    pub(crate) is_active: bool,

    pub(crate) needs_layout: bool,

    /// Any descendant is active.
    has_active: bool,

    /// Any descendant has requested an animation frame.
    pub(crate) request_anim: bool,

    /// Any descendant has requested a timer.
    ///
    /// Note: we don't have any way of clearing this request, as it's
    /// likely not worth the complexity.
    pub(crate) request_timer: bool,

    pub(crate) focus_chain: Vec<WidgetId>,
    pub(crate) request_focus: Option<FocusChange>,
    pub(crate) children: Bloom<WidgetId>,
    pub(crate) children_changed: bool,
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

impl<T, W: Widget<T>> WidgetPod<T, W> {
    /// Create a new widget pod.
    ///
    /// In a widget hierarchy, each widget is wrapped in a `WidgetPod`
    /// so it can participate in layout and event flow. The process of
    /// adding a child widget to a container should call this method.
    pub fn new(inner: W) -> WidgetPod<T, W> {
        let mut state = BaseState::new(inner.id().unwrap_or_else(WidgetId::next));
        state.children_changed = true;
        state.needs_layout = true;
        WidgetPod {
            state,
            old_data: None,
            env: None,
            inner,
        }
    }

    /// Read-only access to state. We don't mark the field as `pub` because
    /// we want to control mutation.
    pub(crate) fn state(&self) -> &BaseState {
        &self.state
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
        self.state.id
    }

    /// Set layout rectangle.
    ///
    /// Intended to be called on child widget in container's `layout`
    /// implementation.
    pub fn set_layout_rect(&mut self, layout_rect: Rect) {
        self.state.layout_rect = layout_rect;
    }

    #[deprecated(since = "0.5.0", note = "use layout_rect() instead")]
    #[doc(hidden)]
    pub fn get_layout_rect(&self) -> Rect {
        self.state.layout_rect
    }

    /// The layout rectangle.
    ///
    /// This will be same value as set by `set_layout_rect`.
    pub fn layout_rect(&self) -> Rect {
        self.state.layout_rect
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
    /// [`layout`]: widget/trait.Widget.html#tymethod.layout
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
    /// [`layout`]: widget/trait.Widget.html#tymethod.layout
    /// [`Insets`]: struct.Insets.html
    pub fn compute_parent_paint_insets(&self, parent_size: Size) -> Insets {
        let parent_bounds = Rect::ZERO.with_size(parent_size);
        let union_pant_rect = self.paint_rect().union(parent_bounds);
        union_pant_rect - parent_bounds
    }
}

impl<T: Data, W: Widget<T>> WidgetPod<T, W> {
    /// Paint a child widget.
    ///
    /// Generally called by container widgets as part of their [`paint`]
    /// method.
    ///
    /// Note that this method does not apply the offset of the layout rect.
    /// If that is desired, use [`paint_with_offset`] instead.
    ///
    /// [`layout`]: widget/trait.Widget.html#tymethod.layout
    /// [`paint`]: widget/trait.Widget.html#tymethod.paint
    /// [`paint_with_offset`]: #method.paint_with_offset
    pub fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut ctx = PaintCtx {
            render_ctx: paint_ctx.render_ctx,
            window_id: paint_ctx.window_id,
            z_ops: Vec::new(),
            region: paint_ctx.region.clone(),
            base_state: &self.state,
            focus_widget: paint_ctx.focus_widget,
        };
        self.inner.paint(&mut ctx, data, &env);
        paint_ctx.z_ops.append(&mut ctx.z_ops);

        if env.get(Env::DEBUG_PAINT) {
            let rect = Rect::from_origin_size(Point::ORIGIN, ctx.size());
            let id = self.id().to_raw();
            let color = env.get_debug_color(id);
            ctx.stroke(rect, &color, 1.0);
        }

        self.state.needs_inval = false;
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
        if !paint_if_not_visible && !paint_ctx.region().intersects(self.state.paint_rect()) {
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
        layout_ctx.paint_insets = Insets::ZERO;
        let size = self.inner.layout(layout_ctx, bc, data, &env);
        self.state.paint_insets = layout_ctx.paint_insets;
        self.state.needs_layout = false;
        size
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
        if self.old_data.is_none() {
            log::error!(
                "widget {:?} is receiving an event without having first \
                 received WidgetAdded.",
                ctx.widget_id()
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
        let mut child_ctx = EventCtx {
            cursor: ctx.cursor,
            command_queue: ctx.command_queue,
            window: &ctx.window,
            window_id: ctx.window_id,
            base_state: &mut self.state,
            had_active,
            is_handled: false,
            is_root: false,
            focus_widget: ctx.focus_widget,
        };
        let rect = child_ctx.base_state.layout_rect;
        // Note: could also represent this as `Option<Event>`.
        let mut recurse = true;
        let mut hot_changed = None;
        let child_event = match event {
            Event::WindowConnected => Event::WindowConnected,
            Event::Size(size) => {
                child_ctx.request_layout();
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
                recurse = child_ctx.has_focus();
                Event::KeyDown(*e)
            }
            Event::KeyUp(e) => {
                recurse = child_ctx.has_focus();
                Event::KeyUp(*e)
            }
            Event::Paste(e) => {
                recurse = child_ctx.has_focus();
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
            Event::Timer(id) => {
                recurse = child_ctx.base_state.request_timer;
                Event::Timer(*id)
            }
            Event::Command(cmd) => Event::Command(cmd.clone()),
            Event::TargetedCommand(target, cmd) => match target {
                Target::Window(_) => Event::Command(cmd.clone()),
                Target::Widget(id) if *id == child_ctx.widget_id() => Event::Command(cmd.clone()),
                Target::Widget(id) => {
                    recurse = child_ctx.base_state.children.contains(id);
                    Event::TargetedCommand(*target, cmd.clone())
                }
                Target::Global => panic!("Target::Global should be converted before WidgetPod"),
            },
        };
        if let Some(is_hot) = hot_changed {
            let hot_changed_event = LifeCycle::HotChanged(is_hot);
            let mut lc_ctx = child_ctx.make_lifecycle_ctx();
            self.inner
                .lifecycle(&mut lc_ctx, &hot_changed_event, data, &env);
        }
        if recurse {
            child_ctx.base_state.has_active = false;
            self.inner.event(&mut child_ctx, &child_event, data, &env);
            child_ctx.base_state.has_active |= child_ctx.base_state.is_active;
        };

        ctx.base_state.merge_up(&child_ctx.base_state);
        ctx.is_handled |= child_ctx.is_handled;
    }

    pub fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let recurse = match event {
            LifeCycle::AnimFrame(_) => {
                let r = self.state.request_anim;
                self.state.request_anim = false;
                r
            }
            LifeCycle::WidgetAdded => {
                assert!(self.old_data.is_none());

                self.old_data = Some(data.clone());
                self.env = Some(env.clone());

                true
            }
            LifeCycle::RouteWidgetAdded => {
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
            LifeCycle::HotChanged(_) => false,
            LifeCycle::RouteFocusChanged { old, new } => {
                self.state.request_focus = None;

                let this_changed = if *old == Some(self.state.id) {
                    Some(false)
                } else if *new == Some(self.state.id) {
                    Some(true)
                } else {
                    None
                };

                if let Some(change) = this_changed {
                    let event = LifeCycle::FocusChanged(change);
                    self.inner.lifecycle(ctx, &event, data, env);
                    false
                } else {
                    old.map(|id| self.state.children.contains(&id))
                        .or_else(|| new.map(|id| self.state.children.contains(&id)))
                        .unwrap_or(false)
                }
            }
            LifeCycle::FocusChanged(_) => {
                self.state.request_focus = None;
                true
            }
            #[cfg(test)]
            LifeCycle::DebugRequestState { widget, state_cell } => {
                if *widget == self.id() {
                    state_cell.set(self.state.clone());
                    false
                } else {
                    self.state.children.contains(&widget)
                }
            }
            #[cfg(test)]
            LifeCycle::DebugInspectState(f) => {
                f.call(&self.state);
                true
            }
        };

        let mut child_ctx = LifeCycleCtx {
            command_queue: ctx.command_queue,
            base_state: &mut self.state,
            window_id: ctx.window_id,
        };

        if recurse {
            self.inner.lifecycle(&mut child_ctx, event, data, env);
        }

        ctx.base_state.merge_up(&self.state);

        // we need to (re)register children in case of one of the following events
        match event {
            LifeCycle::WidgetAdded | LifeCycle::RouteWidgetAdded => {
                self.state.children_changed = false;
                ctx.base_state.children = ctx.base_state.children.union(self.state.children);
                ctx.base_state.focus_chain.extend(&self.state.focus_chain);
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
    /// [`update`]: trait.Widget.html#method.update
    pub fn update(&mut self, ctx: &mut UpdateCtx, data: &T, env: &Env) {
        match (self.old_data.as_ref(), self.env.as_ref()) {
            (Some(d), Some(e)) if d.same(data) && e.same(env) => return,
            (None, _) => {
                log::warn!("old_data missing in {:?}, skipping update", self.id());
                self.old_data = Some(data.clone());
                self.env = Some(env.clone());
                return;
            }
            _ => (),
        }

        let mut child_ctx = UpdateCtx {
            window: ctx.window,
            base_state: &mut self.state,
            window_id: ctx.window_id,
        };

        self.inner
            .update(&mut child_ctx, self.old_data.as_ref().unwrap(), data, env);
        self.old_data = Some(data.clone());
        self.env = Some(env.clone());

        ctx.base_state.merge_up(&self.state)
    }
}

impl<T, W: Widget<T> + 'static> WidgetPod<T, W> {
    /// Box the contained widget.
    ///
    /// Convert a `WidgetPod` containing a widget of a specific concrete type
    /// into a dynamically boxed widget.
    pub fn boxed(self) -> BoxedWidget<T> {
        WidgetPod::new(Box::new(self.inner))
    }
}

impl BaseState {
    pub(crate) fn new(id: WidgetId) -> BaseState {
        BaseState {
            id,
            layout_rect: Rect::ZERO,
            paint_insets: Insets::ZERO,
            needs_inval: false,
            is_hot: false,
            needs_layout: false,
            is_active: false,
            has_active: false,
            request_anim: false,
            request_timer: false,
            request_focus: None,
            focus_chain: Vec::new(),
            children: Bloom::new(),
            children_changed: false,
        }
    }

    /// Update to incorporate state changes from a child.
    fn merge_up(&mut self, child_state: &BaseState) {
        self.needs_inval |= child_state.needs_inval;
        self.needs_layout |= child_state.needs_layout;
        self.request_anim |= child_state.request_anim;
        self.request_timer |= child_state.request_timer;
        self.has_active |= child_state.has_active;
        self.children_changed |= child_state.children_changed;
        self.request_focus = self.request_focus.or(child_state.request_focus);
    }

    #[inline]
    pub(crate) fn size(&self) -> Size {
        self.layout_rect.size()
    }

    /// The paint region for this widget.
    ///
    /// For more information, see [`WidgetPod::paint_rect`].
    ///
    /// [`WidgetPod::paint_rect`]: struct.WidgetPod.html#method.paint_rect
    pub(crate) fn paint_rect(&self) -> Rect {
        self.layout_rect + self.paint_insets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Flex, Scroll, Split, TextBox, WidgetExt};
    use crate::WindowId;

    const ID_1: WidgetId = WidgetId::reserved(0);
    const ID_2: WidgetId = WidgetId::reserved(1);
    const ID_3: WidgetId = WidgetId::reserved(2);

    #[test]
    fn register_children() {
        fn make_widgets() -> impl Widget<Option<u32>> {
            Split::vertical(
                Flex::<Option<u32>>::row()
                    .with_child(TextBox::new().with_id(ID_1).parse(), 1.0)
                    .with_child(TextBox::new().with_id(ID_2).parse(), 1.0)
                    .with_child(TextBox::new().with_id(ID_3).parse(), 1.0),
                Scroll::new(TextBox::new().parse()),
            )
        }

        let widget = make_widgets();
        let mut widget = WidgetPod::new(widget).boxed();

        let mut command_queue: CommandQueue = VecDeque::new();
        let mut state = BaseState::new(WidgetId::next());
        let mut ctx = LifeCycleCtx {
            command_queue: &mut command_queue,
            base_state: &mut state,
            window_id: WindowId::next(),
        };

        let env = Env::default();

        widget.lifecycle(&mut ctx, &LifeCycle::WidgetAdded, &None, &env);
        assert!(ctx.base_state.children.contains(&ID_1));
        assert!(ctx.base_state.children.contains(&ID_2));
        assert!(ctx.base_state.children.contains(&ID_3));
        assert_eq!(ctx.base_state.children.entry_count(), 7);
    }
}
