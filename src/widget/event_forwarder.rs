// Copyright 2018 The xi-editor Authors.
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

//! Widget for just forwarding events to a listener.

use std::any::Any;
use std::marker::PhantomData;

use widget::Widget;
use {HandlerCtx, Id, Ui};

pub struct EventForwarder<T>(PhantomData<T>);

impl<T: Any + Clone> EventForwarder<T> {
    pub fn new() -> Self {
        EventForwarder(Default::default())
    }

    pub fn ui(self, child: Id, ctx: &mut Ui) -> Id {
        ctx.add(self, &[child])
    }
}

impl<T: Any + Clone> Widget for EventForwarder<T> {
    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        if let Some(event) = payload.downcast_ref::<T>() {
            ctx.send_event(event.clone());
            true
        } else {
            false
        }
    }
}
