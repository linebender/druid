// Copyright 2019 The xi-editor Authors.
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

use druid::widget::prelude::*;
use druid::widget::BackgroundBrush;
use druid::{Data, Point, Rect, Selector, SingleUse, WidgetPod};

/// A widget that has a child, and can optionally show modal widgets that obscure the child.
pub(crate) struct ModalHost<T, W> {
    child: W,
    /// A stack of modal widgets. Only the top widget on the stack gets user interaction events.
    modals: Vec<Modal<T>>,
}

/// Describes a modal widget.
///
/// A modal widget is a widget that can be displayed over all the other widgets in a window. It
/// consists of a widget (which must take the same data type as the [`Window`]) and some settings
/// describing how the widget will be presented.
///
/// You can display a modal widget by sending a [`SHOW_MODAL`] command to a window.
///
/// [`Window`]: struct.Window.html
/// [`SHOW_MODAL`]: struct.Modal.html#associatedconstant.SHOW_MODAL
pub struct Modal<T> {
    widget: WidgetPod<T, Box<dyn Widget<T>>>,
    /// If false, only the modal will get user input events.
    pass_through_events: bool,
    /// If set, a background that will be drawn over the `ModalHost` before drawing the modal.
    background: Option<BackgroundBrush<T>>,
    /// If set, the origin of the modal widget. If unset, the modal widget is centered in the
    /// `ModalHost`.
    position: Option<Point>,
}

impl Modal<()> {
    /// Command to dismiss the modal.
    pub const DISMISS_MODAL: Selector<()> = Selector::new("druid.dismiss-modal-widget");
}

impl<T> Modal<T> {
    /// Command to display a modal in this host.
    ///
    /// Note: this is a bit of a footgun, because the typed selectors don't know about generics. In
    /// particular, this means that if you submit a SHOW_MODAL command with the wrong `T`, it will
    /// type-check but panic at run-time.
    pub const SHOW_MODAL: Selector<SingleUse<Modal<T>>> = Selector::new("druid.show-modal-widget");

    /// Creates a new modal for showing the widget `innner`.
    pub fn new(inner: impl Widget<T> + 'static) -> Modal<T> {
        Modal {
            widget: WidgetPod::new(Box::new(inner)),
            pass_through_events: false,
            background: None,
            position: None,
        }
    }

    /// Sets the background for this modal.
    ///
    /// This background will be drawn on top of the window, but below the modal widget.
    pub fn background<B: Into<BackgroundBrush<T>>>(mut self, background: B) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Determines whether to pass through events from the modal to the rest of the window.
    ///
    /// The default value of `pass_through` is `false`, meaning that the user can only interact
    /// with the modal widget.
    pub fn pass_through_events(mut self, pass_through: bool) -> Self {
        self.pass_through_events = pass_through;
        self
    }

    /// Sets the origin of the modal widget, relative to the window.
    ///
    /// By default, the modal widget is centered in the window.
    pub fn position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }
}

impl<T, W> ModalHost<T, W> {
    pub(crate) fn new(widget: W) -> Self {
        ModalHost {
            child: widget,
            modals: Vec::new(),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for ModalHost<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(Modal::<T>::SHOW_MODAL) => {
                if let Some(modal) = cmd.get_unchecked(Modal::<T>::SHOW_MODAL).take() {
                    self.modals.push(modal);
                    ctx.children_changed();
                } else {
                    log::error!("couldn't get modal payload");
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(Modal::DISMISS_MODAL) => {
                if self.modals.pop().is_some() {
                    ctx.children_changed();
                } else {
                    log::warn!("cannot dismiss modal; no modal shown");
                }
                ctx.set_handled();
            }

            // User input gets delivered to the top of the modal stack, passing through every modal
            // that wants to pass through events.
            e if is_user_input(e) => {
                let mut done = false;
                for modal in self.modals.iter_mut().rev() {
                    modal.widget.event(ctx, event, data, env);
                    done |= !modal.pass_through_events;
                    if done {
                        break;
                    }
                }
                if !done {
                    self.child.event(ctx, event, data, env);
                }
            }
            // Other events (timers, commands) are delivered to everything.
            other => {
                for modal in &mut self.modals {
                    modal.widget.event(ctx, other, data, env);
                }
                self.child.event(ctx, other, data, env);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for modal in &mut self.modals {
            modal.widget.lifecycle(ctx, event, data, env);
        }
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        for modal in &mut self.modals {
            modal.widget.update(ctx, data, env);
        }
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        for modal in &mut self.modals {
            let modal_constraints = BoxConstraints::new(Size::ZERO, size);
            let modal_size = modal.widget.layout(ctx, &modal_constraints, data, env);
            let modal_origin = if let Some(pos) = modal.position {
                // TODO: translate the position to ensure that the modal fits in our bounds.
                pos
            } else {
                ((size.to_vec2() - modal_size.to_vec2()) / 2.0).to_point()
            };
            let modal_frame = Rect::from_origin_size(modal_origin, modal_size);
            modal.widget.set_layout_rect(ctx, data, env, modal_frame);
        }
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
        for modal in &mut self.modals {
            let frame = ctx.size().to_rect();
            if let Some(bg) = &mut modal.background {
                ctx.with_save(|ctx| {
                    ctx.clip(frame);
                    bg.paint(ctx, data, env);
                });
            }

            // TODO: cmyr's modal stuff had support for a drop-shadow
            /*
            let modal_rect = modal.layout_rect() + Vec2::new(5.0, 5.0);
            let blur_color = Color::grey8(100);
            ctx.blurred_rect(modal_rect, 5.0, &blur_color);
            */
            modal.widget.paint(ctx, data, env);
        }
    }
}

fn is_user_input(event: &Event) -> bool {
    match event {
        Event::MouseUp(_)
        | Event::MouseDown(_)
        | Event::MouseMove(_)
        | Event::KeyUp(_)
        | Event::KeyDown(_)
        | Event::Paste(_)
        | Event::Wheel(_)
        | Event::Zoom(_) => true,
        _ => false,
    }
}
