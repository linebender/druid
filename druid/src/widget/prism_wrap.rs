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
        dbg!("event", &event);
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
        dbg!("update");
        let inner = &mut self.inner;
        let prism = &self.prism;

        #[allow(clippy::blocks_in_if_conditions)]
        if prism
            .with(data, |newer_data| {
                if prism
                    .with(old_data, |older_data| {
                        // this means this variant is currently active
                        // and also was previously active as well
                        //
                        // (has both old and new data)
                        if !old_data.same(data) {
                            // forwards older and newer data into inner
                            dbg!("+old +new");
                            inner.update(ctx, older_data, newer_data, env);
                        }
                    })
                    .is_none()
                {
                    // doesn't have an old_data,
                    // so this variant just got activated

                    dbg!("-old +new");
                    ctx.children_changed();
                    inner.update(ctx, newer_data, newer_data, env);
                    // ctx.request_layout(); // variant was changed
                    // ctx.request_paint(); // variant was changed
                    // inner.update(ctx, newer_data, newer_data, env);
                }
            })
            .is_none()
        {
            // this means the new_data is missing,
            // so maybe this variant just got de-activated,
            // or it was never active.
            //
            // check to see if it was just de-activated,
            // or was never active:
            #[allow(clippy::single_match)]
            if prism
                .with(old_data, |_older_data| {
                    // this means it just got de-activated.

                    dbg!("+old -new");
                    ctx.children_changed();
                    // ctx.request_layout(); // variant was changed
                    // ctx.request_paint(); // variant was changed
                    inner.update(ctx, _older_data, _older_data, env);
                    // inner.update(ctx, _older_data, _older_data, env);
                })
                .is_none()
            {
                // this means it was never active.
                {
                    dbg!("-old -new");
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T1, env: &Env) -> Size {
        dbg!("layout");
        // if self.new_variant {
        let inner = &mut self.inner;
        self.prism
            .with::<Size, _>(data, |data| inner.layout(ctx, bc, data, env))
            .unwrap_or_else(|| bc.min())
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self.prism.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
