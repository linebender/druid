// Copyright 2021 The Druid Authors.
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

//! A widget that provides simple visual styling options to a child.

use crate::widget::SingleChildContainer;
use crate::Widget;

/// A widget that takes an arbitrary closure, and updates a child widget.
pub struct Updater<W> {
    inner: W,
    update_fn: Box<dyn FnMut(&mut W)>,
}

impl<W> Updater<W> {
    /// Create an updater with a child
    pub fn new(inner: W, f: impl FnMut(&mut W) + 'static) -> Self {
        Self {
            inner,
            update_fn: Box::new(f),
        }
    }
}

impl<W: Widget> SingleChildContainer for Updater<W> {
    type Child = W;

    fn widget(&self) -> &Self::Child {
        &self.inner
    }

    fn widget_mut(&mut self) -> &mut Self::Child {
        &mut self.inner
    }

    fn update(&mut self) {
        (self.update_fn)(&mut self.inner);
        self.inner.update()
    }
}
