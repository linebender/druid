use crate::event::Event::{MouseDown, MouseMove, MouseUp, Wheel};
use crate::{
    Affine, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, MouseEvent,
    PaintCtx, RenderContext, Size, UpdateCtx, Vec2, Widget,
};
use std::f64::consts::PI;

pub struct Rotated<W> {
    child: W,
    quarter_turns: u8,
    transforms: Option<(Affine, Affine)>,
}

impl<W> Rotated<W> {
    pub fn new(child: W, quarter_turns: u8) -> Self {
        Rotated {
            child,
            quarter_turns,
            transforms: None,
        }
    }
}

impl<W> Rotated<W> {
    fn flip(&self, size: Size) -> Size {
        if self.quarter_turns % 2 == 0 {
            size
        } else {
            Size::new(size.height, size.width)
        }
    }

    fn affine(&self, child_size: Size, my_size: Size) -> Affine {
        let a = ((self.quarter_turns % 4) as f64) * PI / 2.0;

        Affine::translate(Vec2::new(my_size.width / 2., my_size.height / 2.))
            * Affine::rotate(a)
            * Affine::translate(Vec2::new(-child_size.width / 2., -child_size.height / 2.))
    }

    fn translate_mouse_event(&self, inverse: Affine, me: &MouseEvent) -> MouseEvent {
        let mut me = me.clone();
        me.pos = inverse * me.pos;
        me
    }
}

impl<T, W: Widget<T>> Widget<T> for Rotated<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some((transform, inverse)) = self.transforms {
            ctx.widget_state.invalid.transform_by(inverse);
            match event {
                MouseMove(me) => self.child.event(
                    ctx,
                    &Event::MouseMove(self.translate_mouse_event(inverse, me)),
                    data,
                    env,
                ),
                MouseDown(me) => self.child.event(
                    ctx,
                    &Event::MouseDown(self.translate_mouse_event(inverse, me)),
                    data,
                    env,
                ),
                MouseUp(me) => self.child.event(
                    ctx,
                    &Event::MouseUp(self.translate_mouse_event(inverse, me)),
                    data,
                    env,
                ),
                Wheel(me) => self.child.event(
                    ctx,
                    &Event::Wheel(self.translate_mouse_event(inverse, me)),
                    data,
                    env,
                ),
                _ => self.child.event(ctx, event, data, env),
            }
            ctx.widget_state.invalid.transform_by(transform);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let bc = BoxConstraints::new(self.flip(bc.min()), self.flip(bc.max()));

        let child_size = self.child.layout(ctx, &bc, data, env);
        let flipped_size = self.flip(child_size);
        let transform = self.affine(child_size, flipped_size);
        self.transforms = Some((transform, transform.inverse()));
        flipped_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some((transform, inverse)) = self.transforms {
            ctx.region.transform_by(inverse);
            ctx.with_save(|ctx| {
                ctx.transform(transform);
                self.child.paint(ctx, data, env)
            });
            ctx.region.transform_by(transform);
        }
    }
}
