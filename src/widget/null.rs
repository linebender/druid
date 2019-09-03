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

//! A null widget, used as a placeholder after deletion.

use std::any::Any;

use log::error;

use crate::{widget::Widget, HandlerCtx};

pub struct NullWidget;

impl Widget for NullWidget {
    fn poke(&mut self, _payload: &mut dyn Any, _ctx: &mut HandlerCtx) -> bool {
        warn!("Poke to null widget: probable use-after-free");
        true
    }
}
