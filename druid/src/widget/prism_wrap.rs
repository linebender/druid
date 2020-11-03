#![allow(missing_docs)]

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

                ctx.children_changed();

                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed
                inner.update(ctx, newer_data, newer_data, env);
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
                ctx.children_changed();

                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed

                inner.update(ctx, _older_data, _older_data, env);
                // inner.update(ctx, _older_data, _older_data, env);
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
