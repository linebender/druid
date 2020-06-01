//! A widget that may present a modal.

use druid::widget::prelude::*;
use druid::widget::BackgroundBrush;
use druid::{Data, Point, Rect, Selector, SingleUse, WidgetPod};

/// A widget that has a child, and can optionally show modal widgets that obscure the child.
pub struct ModalHost<T, W> {
    child: WidgetPod<T, W>,
    /// A stack of modal widgets. Only the top widget on the stack gets user interaction events.
    modals: Vec<Modal<T>>,
}

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

    pub fn new(inner: impl Widget<T> + 'static) -> Modal<T> {
        Modal {
            widget: WidgetPod::new(Box::new(inner)),
            pass_through_events: false,
            background: None,
            position: None,
        }
    }

    pub fn background<B: Into<BackgroundBrush<T>>>(mut self, background: B) -> Self {
        self.background = Some(background.into());
        self
    }
}

impl<T, W: Widget<T>> ModalHost<T, W> {
    pub fn new(widget: W) -> Self {
        ModalHost {
            child: WidgetPod::new(widget),
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

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for modal in &mut self.modals {
            modal.widget.update(ctx, data, env);
        }
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_layout_rect(ctx, data, env, size.to_rect());
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
