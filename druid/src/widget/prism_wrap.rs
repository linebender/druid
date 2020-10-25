#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]

use crate::kurbo::Size;
use crate::optics::PartialPrism;
use crate::widget::prelude::*;
use crate::Data;
use std::marker::PhantomData;

pub struct PrismWrap<U, P, W> {
    inner: W,
    prism: P,
    // The following is a workaround for otherwise getting E0207.
    phantom: PhantomData<U>,
}

impl<U, P, W> PrismWrap<U, P, W> {
    pub fn new(inner: W, prism: P) -> PrismWrap<U, P, W> {
        PrismWrap {
            inner,
            prism,
            phantom: PhantomData,
        }
    }
}

impl<T1, T2, P, W> Widget<T1> for PrismWrap<T2, P, W>
where
    T1: Data,
    T2: Data,
    P: PartialPrism<T1, T2>,
    W: Widget<T2>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T1, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with_mut::<(), _>(data, |data| inner.event(ctx, event, data, env));
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with::<(), _>(data, |data| inner.lifecycle(ctx, event, data, env));
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T1, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        let prism = &self.prism;

        #[allow(clippy::blocks_in_if_conditions)]
        match prism.with(data, |newer_data| {
            if prism
                .with(old_data, |older_data| {
                    if !old_data.same(data) {
                        // forwards older and newer data into inner
                        inner.update(ctx, older_data, newer_data, env);
                    }
                })
                .is_none()
            {
                // this is when this variant just got activated
                // ie. does not have an old_data

                // ctx.children_changed();
                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed
                // inner.update(ctx, newer_data, newer_data, env);
                // inner.update(ctx, newer_data, newer_data, env);
            }
        }) {
            // already had the newer data,
            // with or without older data.
            // do nothing more
            Some(()) => (),
            // didn't have the newer data,
            // check if at least the older data is available
            #[allow(clippy::single_match)]
            None => match prism.with(old_data, |_older_data| {
                // only had the older data
                // send older as both older and newer
                // TODO: check if this is right
                // maybe just ignore the inner update call..
                // ctx.children_changed();
                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed

                // inner.update(ctx, older_data, older_data, env);
                // inner.update(ctx, older_data, older_data, env);
            }) {
                // already had only the older data,
                // do nothing more.
                Some(()) => (),
                // didn't have any of the older nor newer data,
                // do nothing.
                // TODO: check if this is right
                None => {}
            },
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T1, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.prism
            .with::<Size, _>(data, |data| inner.layout(ctx, bc, data, env))
            .unwrap_or(Size::ZERO)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self.prism.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

macro_rules! prisms_impl {
    ($name:ident ( $($idx:literal),+ ) ) => {
        paste::paste! {

            // start of $name def
            /// Container for prisms that are based on
            /// the same data type.
            pub struct $name
            <
                // the outer data
                T0,
                // T1, T2, ..
                // each variant's inner data
                $([<T $idx>],)+
                // P1, P2, ..
                // each prism (T0 -> T1, T0 -> T2, ..)
                $([<P $idx>],)+
                // W1, W2, ..
                // each wrapping widget
                $([<W $idx>],)+
            > {
                // w1: PrismWrap<T1, P1, W1>,
                // w2: PrismWrap<T2, P2, W2>,
                // each field is a prismwrap widget
                // with a correct data, prism and inner widget connected
                $([<w $idx>]: PrismWrap<[<T $idx>], [<P $idx>], [<W $idx>]>,
                )+
                _marker: std::marker::PhantomData<T0>,
            }
            // end of $name def

            // start of $name impl
            impl
            < // same as before
                T0, $([<T $idx>],)+ $([<P $idx>],)+ $([<W $idx>],)+
            > $name
            < // same as before
                T0, $([<T $idx>],)+ $([<P $idx>],)+ $([<W $idx>],)+
            > {
                pub fn new(
                    // same as the struct fields
                    $([<w $idx>]: PrismWrap<[<T $idx>], [<P $idx>], [<W $idx>]>,
                    )+
                ) -> Self {
                    Self {
                        // w1, w2, ..
                        $([<w $idx>],
                        )+
                        _marker: std::marker::PhantomData,
                    }
                }
            }
            // end of $name impl

            // start of Widget impl
            impl
            < // same as before
                T0, $([<T $idx>],)+ $([<P $idx>],)+ $([<W $idx>],)+
            > Widget<T0> for $name
            < // same as before
                T0, $([<T $idx>],)+ $([<P $idx>],)+ $([<W $idx>],)+
            >
            where
                T0: Data,
                // T1: Data, T2: Data, ..
                $([<T $idx>]: Data,
                )+
                // P1: PartialPrism<T0, T1>,
                // P2: PartialPrism<T0, T2>,
                $([<P $idx>]: PartialPrism<T0, [<T $idx>]>,
                )+
                // W1: Widget<T1>,
                // W2: Widget<T2>,
                $([<W $idx>]: Widget<[<T $idx>]>,
                )+
            {
                fn event(
                    &mut self,
                    ctx: &mut ::druid::EventCtx,
                    event: &::druid::Event,
                    data: &mut T0,
                    env: &::druid::Env,
                ) {
                    // self.w1.event(ctx, event, data, env);
                    // self.w2.event(ctx, event, data, env);
                    $(self.[<w $idx>].event(ctx, event, data, env);
                    )+
                }

                fn lifecycle(
                    &mut self,
                    ctx: &mut ::druid::LifeCycleCtx,
                    event: &::druid::LifeCycle,
                    data: &T0,
                    env: &::druid::Env,
                ) {
                    // self.w1.lifecycle(ctx, event, data, env);
                    // self.w2.lifecycle(ctx, event, data, env);
                    $(self.[<w $idx>].lifecycle(ctx, event, data, env);
                    )+
                }

                fn update(&mut self, ctx: &mut ::druid::UpdateCtx, old_data: &T0, data: &T0, env: &::druid::Env) {
                    // self.w1.update(ctx, old_data, data, env);
                    // self.w2.update(ctx, old_data, data, env);
                    $(self.[<w $idx>].update(ctx, old_data, data, env);
                    )+
                }

                fn layout(
                    &mut self,
                    ctx: &mut ::druid::LayoutCtx,
                    bc: &::druid::BoxConstraints,
                    data: &T0,
                    env: &::druid::Env,
                ) -> ::druid::Size {
                    crate::Size::ZERO
                    // self.w1.layout(ctx, bc, data, env) +
                    // self.w2.layout(ctx, bc, data, env)
                    $(+ self.[<w $idx>].layout(ctx, bc, data, env)
                    )+
                }

                fn paint(&mut self, ctx: &mut ::druid::PaintCtx, data: &T0, env: &::druid::Env) {
                    // self.w1.paint(ctx, data, env);
                    // self.w2.paint(ctx, data, env);
                    $(self.[<w $idx>].paint(ctx, data, env);
                    )+
                }
            }
            // end of Widget impl
        }
        // end of paste!
    }
}

prisms_impl! { Prisms1 ( 1 ) }
prisms_impl! { Prisms2 ( 1, 2 ) }
prisms_impl! { Prisms3 ( 1, 2, 3 ) }
prisms_impl! { Prisms4 ( 1, 2, 3, 4 ) }
prisms_impl! { Prisms5 ( 1, 2, 3, 4, 5 ) }
prisms_impl! { Prisms6 ( 1, 2, 3, 4, 5, 6 ) }
prisms_impl! { Prisms7 ( 1, 2, 3, 4, 5, 6, 7 ) }
prisms_impl! { Prisms8 ( 1, 2, 3, 4, 5, 6, 7, 8 ) }
prisms_impl! { Prisms9 ( 1, 2, 3, 4, 5, 6, 7, 8, 9 ) }
prisms_impl! { Prisms10 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 ) }
prisms_impl! { Prisms11 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 ) }
prisms_impl! { Prisms12 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 ) }
prisms_impl! { Prisms13 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13 ) }
prisms_impl! { Prisms14 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 ) }
prisms_impl! { Prisms15 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15 ) }
prisms_impl! { Prisms16 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 ) }
