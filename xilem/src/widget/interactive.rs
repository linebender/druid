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

use std::marker::PhantomData;

use druid_shell::{
    kurbo::{Point, Size},
    piet::{Piet},
};

use crate::{event::Event, id::IdPath, view::interactive::MaybeFrom};
use super::{Widget};


#[derive(Default)]

pub struct Interactive<W: Widget, E> {
    id_path: IdPath,
    child: W,
    phanton: PhantomData<E>
}

impl<W: Widget, E> Interactive<W, E> {
    pub fn new(id_path: &IdPath, child: W) -> Self {
        Interactive {
            id_path: id_path.clone(),
            child,
            phanton: Default::default()
        }
    }
    
    pub fn child_mut(&mut self) -> &mut W {
        &mut self.child
    }
}

impl<'a, W: Widget, E: 'static + MaybeFrom<super::RawEvent>> Widget for Interactive<W, E> {
    fn event(&mut self, cx: &mut super::EventCx, event: &super::RawEvent) {
        match E::maybe_from(event) {
            Some(e) => {
                cx.add_event(Event::new(self.id_path.clone(), e))
            },
            _ => {
                // not an event we are interested in
            }
        }
    }

    fn layout(&mut self, cx: &mut super::LayoutCx, proposed_size: Size) -> Size {
        self.child.layout(cx, proposed_size)
    }
    
    fn paint(&mut self, cx: &mut super::PaintCx) {
        self.child.paint(cx)
    }

    fn lifecycle(&mut self, cx: &mut super::contexts::LifeCycleCx, event: &super::LifeCycle) {
        self.child.lifecycle(cx, event)
    }

    fn update(&mut self, cx: &mut super::UpdateCx) {
        self.child.update(cx)
    }

    fn prelayout(&mut self, cx: &mut super::LayoutCx) -> (Size, Size) {
        self.child.prelayout(cx)
    }
}
