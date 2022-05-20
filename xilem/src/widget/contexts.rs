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

//! Contexts for the widget system.
//!
//! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its contexts.rs.

use std::ops::{Deref, DerefMut};

use druid_shell::{
    kurbo::Point,
    piet::{Piet, PietText, RenderContext},
    WindowHandle,
};

use crate::event::Event;

use super::{
    align::{AlignResult, AlignmentAxis, SingleAlignment},
    PodFlags, WidgetState,
};

// These contexts loosely follow Druid.
pub struct CxState<'a> {
    window: &'a WindowHandle,
    text: PietText,
    events: &'a mut Vec<Event>,
}

pub struct EventCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct LifeCycleCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct UpdateCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct LayoutCx<'a, 'b> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) widget_state: &'a mut WidgetState,
}

pub struct AlignCx<'a> {
    pub(crate) widget_state: &'a WidgetState,
    pub(crate) align_result: &'a mut AlignResult,
    pub(crate) origin: Point,
}

pub struct PaintCx<'a, 'b, 'c> {
    pub(crate) cx_state: &'a mut CxState<'b>,
    pub(crate) piet: &'a mut Piet<'c>,
}

impl<'a> CxState<'a> {
    pub fn new(window: &'a WindowHandle, events: &'a mut Vec<Event>) -> Self {
        CxState {
            window,
            text: window.text(),
            events,
        }
    }

    pub(crate) fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}

impl<'a, 'b> EventCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        EventCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn add_event(&mut self, event: Event) {
        self.cx_state.events.push(event);
    }
}

impl<'a, 'b> LifeCycleCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LifeCycleCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn request_paint(&mut self) {
        self.widget_state.flags |= PodFlags::REQUEST_PAINT;
    }
}

impl<'a, 'b> UpdateCx<'a, 'b> {
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
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
    pub(crate) fn new(cx_state: &'a mut CxState<'b>, root_state: &'a mut WidgetState) -> Self {
        LayoutCx {
            cx_state,
            widget_state: root_state,
        }
    }

    pub fn text(&mut self) -> &mut PietText {
        &mut self.cx_state.text
    }

    pub fn add_event(&mut self, event: Event) {
        self.cx_state.events.push(event);
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
