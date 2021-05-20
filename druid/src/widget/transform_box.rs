use crate::{WidgetPod, Widget, Affine, Vec2, Data, EventCtx, LifeCycle, PaintCtx, BoxConstraints, LifeCycleCtx, Size, LayoutCtx, Event, Env, UpdateCtx, Rect, Insets, Point, RenderContext};
use std::ops::{Add, Sub};
use crate::core::WidgetState;

pub trait TransformPolicy {
    fn get_transform(&mut self, bc: &BoxConstraints, layout: &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size);
}

impl<F: Fn(&BoxConstraints, &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size)> TransformPolicy for F {
    fn get_transform(&mut self, bc: &BoxConstraints, layout: &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size) {
        (self)(bc, layout)
    }
}

pub struct TransformBox<T, F> {
    widget: WidgetPod<T, Box<dyn Widget<T>>>,
    //The affine transform for external points to internal points
    affine_out_in: Affine,
    //The affine transform for internal points to external points
    affine_in_out: Affine,
    need_layout: bool,
    transform: F,
}

impl<T, F> TransformBox<T, F> {
    pub fn with_transform(widget: impl Widget<T> + 'static, transform: F) -> Self {
        TransformBox {
            widget: WidgetPod::new(Box::new(widget)),
            affine_out_in: Default::default(),
            affine_in_out: Default::default(),
            need_layout: true,
            transform,
        }
    }

    pub fn affine(&self) -> Affine {
        self.affine_out_in
    }

    pub fn affine_inv(&self) -> Affine {
        self.affine_in_out
    }

    pub fn transform(&self) -> &F {
        &self.transform
    }

    pub fn set_transform(&mut self, new:  F) {
        self.transform = new;
    }
}

impl<T: Data, F: TransformPolicy> Widget<T> for TransformBox<T, F> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut event = event.to_owned();
        match &mut event {
            Event::MouseDown(me) => {
                if self.need_layout {return}
                me.pos = self.affine_out_in * me.pos;
            }
            Event::MouseUp(me) => {
                if self.need_layout {return}
                me.pos = self.affine_out_in * me.pos;
            }
            Event::MouseMove(me) => {
                if self.need_layout {return}
                me.pos = self.affine_out_in * me.pos;
            }
            Event::Wheel(me) => {
                if self.need_layout {return}
                me.pos = self.affine_out_in * me.pos;
            }
            _ => {}
        }
        self.widget.event(ctx, &event, data, env);
        if !ctx.widget_state.invalid.is_empty() {
            ctx.request_paint();
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.widget.lifecycle(ctx, event, data, env);
        if !ctx.widget_state.invalid.is_empty() {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        self.widget.update(ctx, data, env);
        if !ctx.widget_state.invalid.is_empty() {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let Self {transform, widget, ..} = self;
        let mut child_layout = |bc: &BoxConstraints|widget.layout(ctx, bc, data, env);
        let (affine, size): (Affine, Size) = transform.get_transform(bc, &mut child_layout);

        self.affine_out_in = affine;
        self.affine_in_out = affine.inverse();

        // update the current mouse position to set hot correctly
        ctx.mouse_pos = ctx.mouse_pos.map(|pos|self.affine_out_in * pos);
        self.widget.set_origin(ctx, data, env, Point::ZERO);

        let bounding_box = self.affine_in_out.transform_rect_bbox(self.widget.paint_rect());
        ctx.set_paint_insets(bounding_box - size.to_rect());

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        // This works since the widgets origin is zero.
        ctx.region.set_rect(self.widget.paint_rect());
        ctx.transform(self.affine_out_in);

        self.widget.paint(ctx, data, env);
    }
}

#[derive(Copy, Clone, Data, Default, Eq, PartialEq, Hash)]
pub struct AARotation(u8);

impl AARotation {
    pub const ORIGIN: AARotation = AARotation(0);
    pub const CLOCKWISE: AARotation = AARotation(1);
    pub const HALF_WAY: AARotation = AARotation(2);
    pub const COUNTER_CLOCKWISE: AARotation = AARotation(3);

    pub fn clockwise(self) -> Self {
        Self((self.0 + 1) % 4)
    }

    pub fn counter_clockwise(self) -> Self {
        Self((self.0 + 3) % 4)
    }

    pub fn flipped(self) -> Self {
        Self((self.0 + 2) % 4)
    }

    pub fn is_axis_flipped(self) -> bool {
        self.0 % 2 == 1
    }
}

impl Add for AARotation {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self((self.0 + other.0) % 4)
    }
}

pub type AATransform = (AARotation, bool);

pub type AATransformBox<T> = TransformBox<T, AATransform>;

impl TransformPolicy for AATransform {
    fn get_transform(&mut self, bc: &BoxConstraints, layout: &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size) {
        let bc = if self.0.is_axis_flipped() {
            BoxConstraints::new(Size::new(bc.min().height, bc.min().width),
                                Size::new(bc.max().height, bc.max().width))
        } else {
            *bc
        };
        let mut inner_size = layout(&bc);

        let outer_size = if self.0.is_axis_flipped() {
            Size::new(inner_size.height, inner_size.width)
        } else {
            inner_size
        };

        let transform = Affine::translate(outer_size.to_vec2() / 2.0) *
            Affine::rotate(self.0.0 as f64 * std::f64::consts::PI * 0.5) *
            Affine::translate(-outer_size.to_vec2() / 2.0);

        (transform, outer_size)
    }
}

impl Sub for AARotation {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self((self.0 - other.0 + 4) % 4)
    }
}

impl<T> AATransformBox<T> {
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self::with_transform(inner, (AARotation::ORIGIN, false))
    }

    pub fn rotation(&self) -> AARotation {
        self.transform.0
    }

    pub fn is_flipped(&self) -> bool {
        self.transform.1
    }

    pub fn rotated(mut self, rotation: AARotation) -> Self {
        self.rotate(rotation);
        self
    }

    pub fn flipped_horizontal(mut self) -> Self {
        self.flip_horizontal();
        self
    }

    pub fn flipped_vertical(mut self) -> Self {
        self.flip_vertical();
        self
    }

    pub fn rotate(&mut self, rotation: AARotation) {
        let mut transform = *self.transform();
        transform.0 = transform.0.flipped();
        self.set_transform(transform);
    }

    pub fn flip_horizontal(&mut self) {
        let mut transform = *self.transform();
        transform.1 = !transform.1;
        self.set_transform(transform);
    }

    pub fn flip_vertical(&mut self) {
        let mut transform = *self.transform();
        transform.1 = !transform.1;
        transform.0 = transform.0.flipped();
        self.set_transform(transform);
    }
}

impl TransformPolicy for Affine {
    fn get_transform(&mut self, bc: &BoxConstraints, layout: &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size) {
        (*self, layout(bc))
    }
}