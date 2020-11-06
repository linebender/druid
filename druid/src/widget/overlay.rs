#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

macro_rules! overlay_impl {
    ($name:ident ( $($idx:literal),+ ) ) => {
        paste::paste! {

            // start of $name def
            /// Container that draws widgets as independent layers.
            pub struct $name
            <
                // the outer data
                T0,
                // W1: Widget<T0>, W2: Widget<T0>, ..
                // each wrapped widget
                $([<W $idx>]: Widget<T0>,)+
            >
            {
                // w1: W1,
                // w2: W2,
                $([<w $idx>]: [<W $idx>],
                )+
                _marker: std::marker::PhantomData<T0>,
            }
            // end of $name def

            // start of $name impl
            impl
            < // same as before
                T0, $([<W $idx>],)+
            > $name
            < // same as before
                T0, $([<W $idx>],)+
            >
            where
                $([<W $idx>]: Widget<T0>,)+
            {
                pub fn new(
                    // same as the struct fields
                    $([<w $idx>]: [<W $idx>],
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
                T0, $([<W $idx>],)+
            > Widget<T0> for $name
            < // same as before
                T0, $([<W $idx>],)+
            >
            where
                T0: Data,
                // W1: Widget<T1>,
                // W2: Widget<T2>,
                $([<W $idx>]: Widget<T0>,
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

                    // let last = bc.min(); // gives some weird error
                    let last = Size::ZERO;

                    let (max_width, max_height) = (last.width, last.height);
                    // //
                    // let last = self.w1.layout(ctx, bc, data, env);
                    // let (max_width, max_height) = (last.width.max(max_width), last.height.max(max_height));
                    // //
                    // let last = self.w2.layout(ctx, bc, data, env);
                    // let (max_width, max_height) = (last.width.max(max_width), last.height.max(max_height));
                    // //
                    $(
                        let last = self.[<w $idx>].layout(ctx, bc, data, env);
                        let (max_width, max_height) = (last.width.max(max_width), last.height.max(max_height));
                    )+
                    Size::new(max_width, max_height)
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

overlay_impl! { Overlay1 ( 1 ) }
overlay_impl! { Overlay2 ( 1, 2 ) }
overlay_impl! { Overlay3 ( 1, 2, 3 ) }
overlay_impl! { Overlay4 ( 1, 2, 3, 4 ) }
overlay_impl! { Overlay5 ( 1, 2, 3, 4, 5 ) }
overlay_impl! { Overlay6 ( 1, 2, 3, 4, 5, 6 ) }
overlay_impl! { Overlay7 ( 1, 2, 3, 4, 5, 6, 7 ) }
overlay_impl! { Overlay8 ( 1, 2, 3, 4, 5, 6, 7, 8 ) }
overlay_impl! { Overlay9 ( 1, 2, 3, 4, 5, 6, 7, 8, 9 ) }
overlay_impl! { Overlay10 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 ) }
overlay_impl! { Overlay11 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 ) }
overlay_impl! { Overlay12 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 ) }
overlay_impl! { Overlay13 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13 ) }
overlay_impl! { Overlay14 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 ) }
overlay_impl! { Overlay15 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15 ) }
overlay_impl! { Overlay16 ( 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 ) }
