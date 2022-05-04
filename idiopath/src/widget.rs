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
use std::ops::{Deref, DerefMut};

use bitflags::bitflags;

use druid_shell::kurbo::{Affine, Point, Size};
use druid_shell::piet::{Piet, PietText, RenderContext};
use druid_shell::WindowHandle;

use crate::event::Event;

/// A basic widget trait.
pub trait Widget {
    fn event(&mut self, event: &RawEvent, events: &mut Vec<Event>);

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size;

    fn paint(&mut self, ctx: &mut PaintCx);
}

// These contexts loosely follow Druid.
pub struct CxState<'a> {
    window: &'a WindowHandle,
    text: PietText,
}

pub struct LayoutCx<'a, 'b> {
    cx_state: &'a mut CxState<'b>,
    widget_state: &'a mut WidgetState,
}

pub struct PaintCx<'a, 'b, 'c> {
    cx_state: &'a mut CxState<'b>,
    piet: &'a mut Piet<'c>,
}

bitflags! {
    #[derive(Default)]
    struct PodFlags: u32 {
        const REQUEST_UPDATE = 1;
        const REQUEST_LAYOUT = 2;
        const REQUEST_PAINT = 4;
    }
}

/// A pod that contains a widget (in a container).
pub struct Pod {
    state: WidgetState,
    widget: Box<dyn AnyWidget>,
}

#[derive(Default)]
pub struct WidgetState {
    flags: PodFlags,
    origin: Point,
    size: Size,
    proposed_size: Size,
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

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.deref_mut().layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.deref_mut().paint(cx);
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

impl<'a> CxState<'a> {
    pub fn new(window: &'a WindowHandle, text: PietText) -> Self {
        CxState { window, text }
    }
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }
}

impl<'a, 'b, 'c> PaintCx<'a, 'b, 'c> {
    pub fn new(cx_state: &'a mut CxState<'b>, piet: &'a mut Piet<'c>) -> Self {
        PaintCx { cx_state, piet }
    }

    pub fn with_save(&mut self, f: impl FnOnce(&mut PaintCx)) {
        self.piet.save().unwrap();
        f(self);
        self.piet.restore().unwrap();
    }
}

impl<'c> Deref for PaintCx<'_, '_, 'c> {
    type Target = Piet<'c>;

    fn deref(&self) -> &Self::Target {
        self.piet
    }
}

impl<'c> DerefMut for PaintCx<'_, '_, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.piet
    }
}

impl Pod {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Pod {
            state: Default::default(),
            widget: Box::new(widget),
        }
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    pub fn request_update(&mut self) {
        self.state.flags |= PodFlags::REQUEST_UPDATE;
    }

    pub fn event(&mut self, event: &RawEvent, events: &mut Vec<Event>) {
        self.widget.event(event, events);
    }

    pub fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let mut child_cx = LayoutCx {
            cx_state: cx.cx_state,
            widget_state: &mut self.state,
        };
        let new_size = self.widget.layout(&mut child_cx, proposed_size);
        self.state.size = new_size;
        new_size
    }

    pub fn paint(&mut self, cx: &mut PaintCx) {
        cx.with_save(|cx| {
            cx.piet
                .transform(Affine::translate(self.state.origin.to_vec2()));
            self.widget.paint(cx);
        });
    }
}
