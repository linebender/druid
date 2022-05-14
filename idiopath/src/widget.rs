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
pub mod text;
pub mod vstack;

use std::any::Any;
use std::ops::{Deref, DerefMut};

use bitflags::bitflags;

use druid_shell::kurbo::{Affine, Point, Size};
use druid_shell::piet::{Piet, PietText, RenderContext};
use druid_shell::WindowHandle;

use crate::event::Event;

use self::align::{
    AlignResult, AlignmentAxis, Bottom, Center, HorizAlignment, Leading, SingleAlignment, Top,
    Trailing, VertAlignment,
};

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
    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {}

    fn paint(&mut self, cx: &mut PaintCx);
}

// These contexts loosely follow Druid.
pub struct CxState<'a> {
    window: &'a WindowHandle,
    text: PietText,
    events: &'a mut Vec<Event>,
}

pub struct EventCx<'a, 'b> {
    cx_state: &'a mut CxState<'b>,
    widget_state: &'a mut WidgetState,
}

pub struct UpdateCx<'a, 'b> {
    cx_state: &'a mut CxState<'b>,
    widget_state: &'a mut WidgetState,
}

pub struct LayoutCx<'a, 'b> {
    cx_state: &'a mut CxState<'b>,
    widget_state: &'a mut WidgetState,
}

pub struct AlignCx<'a> {
    widget_state: &'a WidgetState,
    align_result: &'a mut AlignResult,
    origin: Point,
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

        const UPWARD_FLAGS = Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits | Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
    }
}

/// A pod that contains a widget (in a container).
pub struct Pod {
    state: WidgetState,
    widget: Box<dyn AnyWidget>,
}

#[derive(Default, Debug)]
pub struct WidgetState {
    flags: PodFlags,
    origin: Point,
    /// The minimum intrinsic size of the widget.
    min_size: Size,
    /// The maximum intrinsic size of the widget.
    max_size: Size,
    /// The size proposed by the widget's container.
    proposed_size: Size,
    /// The size of the widget.
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
    pub fn new(window: &'a WindowHandle, events: &'a mut Vec<Event>) -> Self {
        CxState {
            window,
            text: window.text(),
            events,
        }
    }
}

impl<'a, 'b> EventCx<'a, 'b> {
    pub fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        EventCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn add_event(&mut self, event: Event) {
        self.cx_state.events.push(event);
    }
}

impl<'a, 'b> UpdateCx<'a, 'b> {
    pub fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        UpdateCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn request_layout(&mut self) {
        self.widget_state.flags |= PodFlags::REQUEST_LAYOUT;
    }
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn text(&mut self) -> &mut PietText {
        &mut self.cx_state.text
    }
}

impl<'a> AlignCx<'a> {
    pub fn aggregate(&mut self, alignment: SingleAlignment, value: f64) {
        let origin_value = match alignment.axis() {
            AlignmentAxis::Horizontal => self.origin.x,
            AlignmentAxis::Vertical => self.origin.y,
        };
        self.align_result.aggregate(alignment, value + origin_value);
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

impl WidgetState {
    fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags & PodFlags::UPWARD_FLAGS;
    }

    fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }

    /// Get alignment value.
    ///
    /// The value is in the coordinate system of the parent widget.
    fn get_alignment(&self, widget: &dyn AnyWidget, alignment: SingleAlignment) -> f64 {
        if alignment.id() == Leading.id() || alignment.id() == Top.id() {
            0.0
        } else if alignment.id() == <Center as HorizAlignment>::id(&Center) {
            match alignment.axis() {
                AlignmentAxis::Horizontal => self.size.width * 0.5,
                AlignmentAxis::Vertical => self.size.height * 0.5,
            }
        } else if alignment.id() == Trailing.id() {
            self.size.width
        } else if alignment.id() == Bottom.id() {
            self.size.height
        } else {
            let mut align_result = AlignResult::default();
            let mut align_cx = AlignCx {
                widget_state: self,
                align_result: &mut align_result,
                origin: self.origin,
            };
            widget.align(&mut align_cx, alignment);
            align_result.reap(alignment)
        }
    }
}

impl Pod {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Pod {
            state: WidgetState {
                flags: PodFlags::INIT_FLAGS,
                ..Default::default()
            },
            widget: Box::new(widget),
        }
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    pub fn request_update(&mut self) {
        self.state.request(PodFlags::REQUEST_UPDATE);
    }

    pub fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.widget.event(cx, event);
    }

    /// Propagate an update cycle.
    pub fn update(&mut self, cx: &mut UpdateCx) {
        if self.state.flags.contains(PodFlags::REQUEST_UPDATE) {
            let mut child_cx = UpdateCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            self.widget.update(&mut child_cx);
            self.state.flags.remove(PodFlags::REQUEST_UPDATE);
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    pub fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        if self.state.flags.contains(PodFlags::REQUEST_LAYOUT) {
            let mut child_cx = LayoutCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            let (min_size, max_size) = self.widget.prelayout(&mut child_cx);
            self.state.min_size = min_size;
            self.state.max_size = max_size;
            // Don't remove REQUEST_LAYOUT here, that will be done in layout.
        }
        (self.state.min_size, self.state.max_size)
    }

    pub fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        if self.state.flags.contains(PodFlags::REQUEST_LAYOUT)
            || proposed_size != self.state.proposed_size
        {
            let mut child_cx = LayoutCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            let new_size = self.widget.layout(&mut child_cx, proposed_size);
            self.state.proposed_size = proposed_size;
            self.state.size = new_size;
            self.state.flags.remove(PodFlags::REQUEST_LAYOUT);
        }
        self.state.size
    }

    /// Propagate alignment query to children.
    ///
    /// This call aggregates all instances of the alignment, so cost may be
    /// proportional to the number of descendants.
    pub fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        let mut child_cx = AlignCx {
            widget_state: &self.state,
            align_result: cx.align_result,
            origin: cx.origin + self.state.origin.to_vec2(),
        };
        self.widget.align(&mut child_cx, alignment);
    }

    pub fn paint(&mut self, cx: &mut PaintCx) {
        cx.with_save(|cx| {
            cx.piet
                .transform(Affine::translate(self.state.origin.to_vec2()));
            self.widget.paint(cx);
        });
    }

    pub fn height_flexibility(&self) -> f64 {
        self.state.max_size.height - self.state.min_size.height
    }

    /// The returned value is in the coordinate space of the parent that
    /// owns this pod.
    pub fn get_alignment(&self, alignment: SingleAlignment) -> f64 {
        self.state.get_alignment(&self.widget, alignment)
    }
}
