use crate::{
    Affine, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx,
    PaintCtx, Point, RenderContext, Size, UpdateCtx, Widget, WidgetPod,
};
use std::ops::{Add, Sub};

/// A trait which decides the size and transform of the inner widget based on the outer BoxConstrains.
pub trait TransformPolicy: Data {
    /// This method is run every time same returns false or layout was requested.
    ///
    /// It can decide which [`BoxConstraints`](crate::BoxConstraints) to pass to the inner widget via layout, to meet
    /// its own box constrains.
    ///
    /// The return value contains the size of the [`TransformBox`](TransformBox) and the [`Affine`](crate::Affine) used to transform
    /// the inner widget (The affine transforms points from the local coordinate-space to the
    /// parents coordinate-space which is equal to how [`RenderContext::transform`](crate::RenderContext::transform) works).
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size);
}

impl<F: Data + Fn(&BoxConstraints, &mut dyn FnMut(&BoxConstraints) -> Size) -> (Affine, Size)>
    TransformPolicy for F
{
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        (self)(bc, layout)
    }
}

/// A widget wrapper which layouts the inner widget according to the provided `TransformPolicy`.
pub struct TransformBox<T, F> {
    widget: WidgetPod<T, Box<dyn Widget<T>>>,
    //The affine transform for internal points to external points
    affine_in_out: Affine,
    //The affine transform for external points to internal points
    affine_out_in: Affine,
    need_layout: bool,
    transform: Option<F>,
    extractor: Option<Box<dyn Fn(&T) -> F>>,
}

impl<T, F> TransformBox<T, F> {
    /// creates a new `TransformBox` with the inner widget and a fixed `TransformPolicy`
    pub fn with_transform(widget: impl Widget<T> + 'static, transform: F) -> Self {
        TransformBox {
            widget: WidgetPod::new(Box::new(widget)),
            affine_in_out: Default::default(),
            affine_out_in: Default::default(),
            need_layout: true,
            transform: Some(transform),
            extractor: None,
        }
    }

    /// creates a new `TransformBox` from the inner widget and a closure to extract `TransformPolicy`
    /// from data.
    pub fn with_extractor(
        widget: impl Widget<T> + 'static,
        extractor: impl Fn(&T) -> F + 'static,
    ) -> Self {
        TransformBox {
            widget: WidgetPod::new(Box::new(widget)),
            affine_in_out: Default::default(),
            affine_out_in: Default::default(),
            need_layout: true,
            transform: None,
            extractor: Some(Box::new(extractor)),
        }
    }

    /// returns the affine created by `TransformPolicy`. It is the transform which transform points
    /// from the local coordinate-space to its parent's coordinate-space. It is also the transform
    /// used by PaintCtx.
    pub fn affine(&self) -> Affine {
        self.affine_in_out
    }

    /// returns the cached inverse of the `TransformBox::affine()`. It is used ot transform points
    /// from the parent's coordinate-space to the local coordinate-space. The event method uses it
    /// to get the local position for MouseEvents.
    pub fn affine_inv(&self) -> Affine {
        self.affine_out_in
    }
}

impl<T: Data, F: TransformPolicy + Data> Widget<T> for TransformBox<T, F> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut event = event.to_owned();
        match &mut event {
            Event::MouseDown(me) => {
                if self.need_layout {
                    return;
                }
                me.pos = self.affine_out_in * me.pos;
            }
            Event::MouseUp(me) => {
                if self.need_layout {
                    return;
                }
                me.pos = self.affine_out_in * me.pos;
            }
            Event::MouseMove(me) => {
                if self.need_layout {
                    return;
                }
                me.pos = self.affine_out_in * me.pos;
            }
            Event::Wheel(me) => {
                if self.need_layout {
                    return;
                }
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
        if let Some(extractor) = &self.extractor {
            self.transform = Some(extractor(data));
        }

        self.widget.lifecycle(ctx, event, data, env);
        if !ctx.widget_state.invalid.is_empty() {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        self.widget.update(ctx, data, env);
        if let Some(extractor) = &self.extractor {
            let new = extractor(data);
            if !new.same(self.transform.as_ref().unwrap()) {
                self.transform = Some(new);
                ctx.request_layout();
            }
        }

        if !ctx.widget_state.invalid.is_empty() {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let Self {
            transform, widget, ..
        } = self;
        let transform = transform.as_ref().unwrap();
        let mut child_layout = |bc: &BoxConstraints| widget.layout(ctx, bc, data, env);
        let (affine, size): (Affine, Size) = transform.get_transform(bc, &mut child_layout);

        self.affine_in_out = affine;
        self.affine_out_in = affine.inverse();

        // update the current mouse position to set hot correctly
        ctx.mouse_pos = ctx.mouse_pos.map(|pos| self.affine_in_out * pos);
        self.widget.set_origin(ctx, data, env, Point::ZERO);

        let bounding_box = self
            .affine_in_out
            .transform_rect_bbox(self.widget.paint_rect());
        ctx.set_paint_insets(bounding_box - size.to_rect());

        self.need_layout = false;

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        // This works since the widgets origin is zero.
        ctx.region.set_rect(self.widget.paint_rect());
        ctx.with_save(|ctx| {
            ctx.transform(self.affine_in_out);

            self.widget.paint(ctx, data, env);
        });
    }
}

/// An axis-aligned rotation, the only possible value are 0°, 90°, 180°, 270°
#[derive(Copy, Clone, Data, Default, Eq, PartialEq, Hash, Debug)]
pub struct AaRotation(u8);

impl AaRotation {
    /// 0°
    pub const ORIGIN: AaRotation = AaRotation(0);
    /// 90°
    pub const CLOCKWISE: AaRotation = AaRotation(1);
    /// 180°
    pub const HALF_WAY: AaRotation = AaRotation(2);
    /// 270°
    pub const COUNTER_CLOCKWISE: AaRotation = AaRotation(3);

    /// turn clockwise 90°
    pub fn clockwise(self) -> Self {
        Self((self.0 + 1) % 4)
    }

    /// turn counter-clockwise 90°
    pub fn counter_clockwise(self) -> Self {
        Self((self.0 + 3) % 4)
    }

    /// turn 180°
    pub fn flipped(self) -> Self {
        Self((self.0 + 2) % 4)
    }

    /// Is 90° or 270° therefore the X-Axis becomes the Y-Axis
    pub fn is_axis_flipped(self) -> bool {
        self.0 % 2 == 1
    }
}

impl Add for AaRotation {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self((self.0 + other.0) % 4)
    }
}

impl Sub for AaRotation {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self((self.0 - other.0 + 4) % 4)
    }
}

/// An axis-aligned transformation (rotation and flipping axis).
#[derive(Copy, Clone, Data, Debug, Eq, PartialEq, Lens)]
pub struct AaTransform {
    rotation: AaRotation,
    #[lens(ignore)]
    flipped_horizontal: bool,
}

impl TransformPolicy for AaTransform {
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        let bc = if self.rotation.is_axis_flipped() {
            BoxConstraints::new(
                Size::new(bc.min().height, bc.min().width),
                Size::new(bc.max().height, bc.max().width),
            )
        } else {
            *bc
        };
        let inner_size = layout(&bc);

        let outer_size = if self.rotation.is_axis_flipped() {
            Size::new(inner_size.height, inner_size.width)
        } else {
            inner_size
        };

        let mut transform = Affine::translate(outer_size.to_vec2() / 2.0)
            * Affine::rotate(self.rotation.0 as f64 * std::f64::consts::PI * 0.5)
            * Affine::translate(-inner_size.to_vec2() / 2.0);

        if self.flipped_horizontal {
            transform = Affine::translate((outer_size.width, 0.0)) * Affine::FLIP_X * transform;
        }

        (transform, outer_size)
    }
}

impl Default for AaTransform {
    fn default() -> Self {
        Self {
            rotation: AaRotation::ORIGIN,
            flipped_horizontal: false,
        }
    }
}

impl AaTransform {
    /// returns the rotation of this transform.
    pub fn get_rotation(self) -> AaRotation {
        self.rotation
    }

    /// The transform can't be turned into identity by rotating.
    pub fn is_flipped(self) -> bool {
        self.flipped_horizontal
    }

    /// A builder-style method to rotate the Transform.
    pub fn rotated(mut self, rotation: AaRotation) -> Self {
        self.rotate(rotation);
        self
    }

    /// A builder-style method to flip the transform horizontally.
    pub fn flipped_horizontal(mut self) -> Self {
        self.flip_horizontal();
        self
    }

    /// A builder-style method to flip the transform vertically.
    pub fn flipped_vertical(mut self) -> Self {
        self.flip_vertical();
        self
    }

    /// Rotate the Transform.
    pub fn rotate(&mut self, rotation: AaRotation) {
        self.rotation = self.rotation + rotation;
    }

    /// Flip the transform horizontally.
    pub fn flip_horizontal(&mut self) {
        self.flipped_horizontal = !self.flipped_horizontal;
    }

    /// Flip the transform vertically.
    pub fn flip_vertical(&mut self) {
        self.rotation = self.rotation.flipped();
        self.flipped_horizontal = !self.flipped_horizontal;
    }
}

/// A `TransformPolicy` which rotates and keeps the widget inside the bounds of the TransformBox.
#[derive(Copy, Clone, Data, Debug)]
pub struct BoundedRotation(f64);

impl BoundedRotation {
    /// creates a BoundedRotation from radians.
    pub fn radians(rad: f64) -> Self {
        Self(rad)
    }

    /// creates a BoundedRotation from degrees.
    pub fn degree(deg: f64) -> Self {
        Self::radians(deg / 180.0 * std::f64::consts::PI)
    }

    /// returns the rotation in radians.
    pub fn to_radians(self) -> f64 {
        self.0
    }

    /// returns the rotation in degrees.
    pub fn to_degrees(self) -> f64 {
        self.0 * 180.0 / std::f64::consts::PI
    }
}

impl TransformPolicy for BoundedRotation {
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        let inner_size = layout(&bc);

        let rotation = Affine::rotate(self.0);
        let bounds = rotation.transform_rect_bbox(inner_size.to_rect());

        // translate the rotation to start at (0, 0).
        let transform = Affine::translate((-bounds.x0, -bounds.y0)) * rotation;
        let size = Size::new(bounds.x1 - bounds.x0, bounds.y1 - bounds.y0);

        (transform, size)
    }
}

/// A `TransformPolicy` which rotates a widget around its center. Parts of the widget will become
/// inaccessible.
#[derive(Copy, Clone, Data, Debug)]
pub struct CenterRotation(f64);

impl CenterRotation {
    /// creates a CenterRotation from radians.
    pub fn radians(rad: f64) -> Self {
        Self(rad)
    }

    /// creates a CenterRotation from degrees.
    pub fn degree(deg: f64) -> Self {
        Self::radians(deg / 180.0 * std::f64::consts::PI)
    }

    /// returns the rotation in radians.
    pub fn to_radians(self) -> f64 {
        self.0
    }

    /// returns the rotation in degrees.
    pub fn to_degrees(self) -> f64 {
        self.0 * 180.0 / std::f64::consts::PI
    }
}

impl TransformPolicy for CenterRotation {
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        let size = layout(bc);
        let half = size.to_vec2() / 2.0;
        let affine = Affine::translate(half) * Affine::rotate(self.0) * Affine::translate(-half);

        (affine, size)
    }
}

/// A `TransformPolicy` which applies a specific `Affine` and tries to fit the bounds of the
/// `TransformBox` bounds around the inner widget. Parts of the widget can become inaccessible.
#[derive(Copy, Clone, Data, Debug)]
pub struct BoundedAffine(pub Affine);

impl TransformPolicy for BoundedAffine {
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        let inner_size = layout(bc);
        let bounds = self.0.transform_rect_bbox(inner_size.to_rect());

        (self.0, bc.constrain((bounds.x1, bounds.y1)))
    }
}

/// A `TransformPolicy` which applies a specific `Affine` and keeps the original size of the inner
/// widget. Parts of the widget can become inaccessible.
#[derive(Copy, Clone, Data, Debug)]
pub struct FreeAffine(pub Affine);

impl TransformPolicy for FreeAffine {
    fn get_transform(
        &self,
        bc: &BoxConstraints,
        layout: &mut dyn FnMut(&BoxConstraints) -> Size,
    ) -> (Affine, Size) {
        (self.0, layout(bc))
    }
}
