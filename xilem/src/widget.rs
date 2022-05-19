// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod align;
pub mod button;
mod contexts;
mod core;
pub mod layout_observer;
mod raw_event;
pub mod text;
pub mod vstack;

use std::any::Any;
use std::ops::DerefMut;

use druid_shell::kurbo::Size;

pub use self::contexts::{AlignCx, CxState, EventCx, LayoutCx, PaintCx, UpdateCx};
pub use self::core::Pod;
pub(crate) use self::core::{PodFlags, WidgetState};
pub use self::raw_event::RawEvent;

use self::align::SingleAlignment;

/// A basic widget trait.
pub trait Widget {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent);

    fn update(&mut self, cx: &mut UpdateCx);

    /// Compute intrinsic sizes.
    ///
    /// This method will be called once on widget creation and then on
    /// REQUEST_UPDATE.
    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size);

    /// Compute size given proposed size.
    ///
    /// The value will be memoized given the proposed size, invalidated
    /// on REQUEST_UPDATE. It can count on prelayout being completed.
    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size;

    /// Query for an alignment.
    ///
    /// This method can count on layout already having been completed.
    #[allow(unused)]
    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {}

    fn paint(&mut self, cx: &mut PaintCx);
}

pub trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<W: Widget + 'static> AnyWidget for W {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Widget for Box<dyn AnyWidget> {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.deref_mut().event(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.deref_mut().update(cx);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.deref_mut().prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.deref_mut().layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.deref_mut().paint(cx);
    }
}

pub trait WidgetTuple {
    fn length(&self) -> usize;

    // Follows Panoramix; rethink to reduce allocation
    // Maybe SmallVec?
    fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget>;
}

macro_rules! impl_widget_tuple {
    ( $n: tt; $( $WidgetType:ident),* ; $( $index:tt ),* ) => {
        impl< $( $WidgetType: AnyWidget ),* > WidgetTuple for ( $( $WidgetType, )* ) {
            fn length(&self) -> usize {
                $n
            }

            fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget> {
                let mut v: Vec<&mut dyn AnyWidget> = Vec::with_capacity(self.length());
                $(
                v.push(&mut self.$index);
                )*
                v
            }

        }
    }
}

impl_widget_tuple!(1; W0; 0);
impl_widget_tuple!(2; W0, W1; 0, 1);
impl_widget_tuple!(3; W0, W1, W2; 0, 1, 2);
impl_widget_tuple!(4; W0, W1, W2, W3; 0, 1, 2, 3);
impl_widget_tuple!(5; W0, W1, W2, W3, W4; 0, 1, 2, 3, 4);
impl_widget_tuple!(6; W0, W1, W2, W3, W4, W5; 0, 1, 2, 3, 4, 5);
impl_widget_tuple!(7; W0, W1, W2, W3, W4, W5, W6; 0, 1, 2, 3, 4, 5, 6);
impl_widget_tuple!(8;
    W0, W1, W2, W3, W4, W5, W6, W7;
    0, 1, 2, 3, 4, 5, 6, 7
);
