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
    fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget>;
}

impl<W0: AnyWidget, W1: AnyWidget> WidgetTuple for (W0, W1) {
    fn length(&self) -> usize {
        2
    }

    fn widgets_mut(&mut self) -> Vec<&mut dyn AnyWidget> {
        let mut v: Vec<&mut dyn AnyWidget> = Vec::with_capacity(self.length());
        v.push(&mut self.0);
        v.push(&mut self.1);
        v
    }
}
