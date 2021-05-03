use crate::{WidgetPod, Widget, Affine, Vec2, Data, EventCtx, LifeCycle, PaintCtx, BoxConstraints, LifeCycleCtx, Size, LayoutCtx, Event, Env, UpdateCtx};
use std::ops::{Add, Sub};
use crate::core::WidgetState;

pub trait WidgetTransform {
    fn get_transform(&mut self, bc: &BoxConstraints, layout: &mut dyn FnMut(BoxConstraints) -> Size) -> (Affine, Size);
}

struct TransformBox<D, T> {
    widget: WidgetPod<D, Box<dyn Widget<T>>>,
    //The affine transform for external points to internal points
    affine_out_in: Affine,
    //The affine transform for internal points to external points
    affine_in_out: Affine,
    need_layout: bool,
    transform: T,
}

impl<D, T> TransformBox<D, T> {
    pub fn new(widget: impl Widget<T>, transform: T) -> Self {
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

    pub fn transform(&self) -> &T {
        &self.transform
    }

    pub fn transform_mut(&mut self) -> &mut T {
        self.need_layout = true;
        &mut self.transform
    }

    fn context_cleanup(&mut self, context: &mut WidgetState) {

    }
}

impl<D: Data, T> Widget<D> for TransformBox<D, T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut event = event.to_owned();
        match &mut event {
            Event::MouseDown(me) => {me.}
            Event::MouseUp(_) => {}
            Event::MouseMove(_) => {}
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        unimplemented!()
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        unimplemented!()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        unimplemented!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        unimplemented!()
    }
}

#[derive(Copy, Clone, Data, Default, Eq, PartialEq, Hash)]
pub struct AARotation(u8);

impl AARotation {
    pub const ORIGIN: AARotation = AARotation(0);
    pub const CLOCKWISE: AARotation = AARotation(0);
    pub const HALF_WAY: AARotation = AARotation(0);
    pub const COUNTER_CLOCKWISE: AARotation = AARotation(0);

    pub fn clockwise(&self) -> Self {
        Self((self.0 + 1) % 4)
    }

    pub fn counter_clockwise(&self) -> Self {
        Self((self.0 + 3) % 4)
    }

    pub fn flipped(&self) -> Self {
        Self((self.0 + 2) % 4)
    }
}

impl Add for AARotation {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self((self.0 + other.0) % 4)
    }
}

impl Sub for AARotation {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self((self.0 - other.0 + 4) % 4)
    }
}

pub struct AATransformBox<T> {
    inner: TransformBox<T>,
    rotation: AARotation,
    flipped_horizontal: bool,
    changed: bool,
}

impl AATransformBox<T> {
    pub fn new(inner: impl Widget<T>) -> Self {
        Self {
            inner: TransformBox::new(inner).expand(),
            rotation: Default::default(),
            flipped_horizontal: false,
            changed: true,
        }
    }

    pub fn rotation(&self) -> AARotation {
        self.rotation
    }

    pub fn is_flipped(&self) -> bool {
        self.flipped_horizontal
    }

    pub fn rotated(mut self, rotation: AARotation) -> Self {
        self.rotate(rotation);
        self
    }

    pub fn flipped_horizontal(mut self) -> Self {
        self.flipp_horizontal();
        self
    }

    pub fn flipped_vertical(mut self) -> Self {
        self.flipp_vertical();
        self
    }

    pub fn rotate(&mut self, rotation: AARotation) {
        let flipped = self.flipped_horizontal;
        let rotation = self.rotation + rotation;
        self.set(rotation, flipped);
    }

    pub fn flip_horizontal(&mut self) {
        let flipped = !self.flipped_horizontal;
        let rotation = self.rotation;
        self.set(rotation, flipped);
    }

    pub fn flip_vertical(&mut self) {
        let flipped = !self.flipped_horizontal;
        let rotation = self.rotation.flipped();
        self.set(rotation, flipped);
    }

    pub fn set(&mut self, rotation: AARotation, flipped_x_axis: bool) {
        self.rotation = rotation;
        self.flipped_horizontal = flipped_x_axis;
        self.changed = true;
    }
}