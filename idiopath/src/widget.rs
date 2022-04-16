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

pub mod button;
pub mod column;

use std::any::Any;
use std::ops::DerefMut;

use druid_shell::kurbo::{Point, Size};
use druid_shell::piet::Piet;

use crate::event::Event;

/// A basic widget trait.
pub trait Widget {
    fn event(&mut self, event: &RawEvent, events: &mut Vec<Event>);

    fn layout(&mut self) -> Size;

    fn paint(&mut self, ctx: &mut Piet, pos: Point);
}

// consider renaming, may get other stuff
#[derive(Default)]
pub struct Geom {
    // probably want id?
    size: Size,
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
    fn event(&mut self, event: &RawEvent, events: &mut Vec<Event>) {
        self.deref_mut().event(event, events);
    }

    fn layout(&mut self) -> Size {
        self.deref_mut().layout()
    }

    fn paint(&mut self, ctx: &mut Piet, pos: Point) {
        self.deref_mut().paint(ctx, pos);
    }
}

#[derive(Debug)]
pub enum RawEvent {
    MouseDown(Point),
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

// Note: the name of the trait should perhaps be changed to WidgetSeq because
// the length may in fact be variable.
impl<W: AnyWidget> WidgetTuple for Vec<W> {
    fn length(&self) -> usize {
        self.len()
    }

    fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget> {
        self.iter_mut().map(|w| {
            let dyn_w: &mut dyn AnyWidget = &mut *w;
            dyn_w
        }).collect()
    }
}
