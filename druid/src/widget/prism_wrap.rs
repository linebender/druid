#![allow(missing_docs)]

use crate::kurbo::Size;
use crate::optics::PartialPrism;
use crate::widget::prelude::*;
use crate::Data;
use std::marker::PhantomData;

pub struct PrismWrap<U, P, W> {
    inner: W,
    prism: P,
    lifecycle_widget_added: bool,
    // The following is a workaround for otherwise getting E0207.
    phantom: PhantomData<U>,
}

impl<U, P, W> PrismWrap<U, P, W> {
    pub fn new(inner: W, prism: P) -> PrismWrap<U, P, W> {
        PrismWrap {
            inner,
            prism,
            lifecycle_widget_added: false,
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
        let prism = &self.prism;
        let inner = &mut self.inner;
        let _opt = prism.with_mut::<(), _>(data, |data| inner.event(ctx, event, data, env));
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T1, env: &Env) {
        let prism = &self.prism;
        let inner = &mut self.inner;
        let lifecycle_widget_added = &mut self.lifecycle_widget_added;

        #[allow(clippy::blocks_in_if_conditions)]
        let _opt = prism.with::<(), _>(data, |data| {
            match event {
                druid::LifeCycle::Internal(druid::InternalLifeCycle::RouteWidgetAdded) => {
                    if !*lifecycle_widget_added {
                        *lifecycle_widget_added = true;
                        inner.lifecycle(ctx, &druid::LifeCycle::WidgetAdded, data, env);
                    };
                }
                druid::LifeCycle::WidgetAdded => {
                    *lifecycle_widget_added = true;
                }
                _ => (),
            };

            inner.lifecycle(ctx, event, data, env)
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T1, data: &T1, env: &Env) {
        let prism = &self.prism;
        let inner = &mut self.inner;
        let lifecycle_widget_added = &self.lifecycle_widget_added;

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
                            inner.update(ctx, older_data, newer_data, env);
                        }
                    })
                    .is_none()
                {
                    // doesn't have an old_data,
                    // so this variant just got activated
                    ctx.children_changed();
                    if *lifecycle_widget_added {
                        inner.update(ctx, newer_data, newer_data, env);
                        // note: widget must be WidgetAdded or else it cannot
                        // be WidgetAdded later, after this update run.
                    }
                }
            })
            .is_none()
        {
            // this means the new_data is missing,
            // so maybe this variant just got de-activated,
            // or it was never active.
            //
            // check to see which case it is:
            #[allow(clippy::single_match)]
            if prism
                .with(old_data, |older_data| {
                    // this means it just got de-activated.
                    ctx.children_changed();
                    assert!(*lifecycle_widget_added);
                    inner.update(ctx, older_data, older_data, env);
                })
                .is_none()
            {
                // this means it was never active.
                {}
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T1, env: &Env) -> Size {
        let prism = &self.prism;
        let inner = &mut self.inner;
        prism
            .with::<Size, _>(data, |data| inner.layout(ctx, bc, data, env))
            .unwrap_or_else(|| bc.min())
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T1, env: &Env) {
        let prism = &self.prism;
        let inner = &mut self.inner;
        let _opt = prism.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
