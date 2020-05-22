// Copyright 2020 The xi-editor Authors.
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

//! Private traits that enable default trait implementations to access struct fields.
//!
//! The traits themselves have to be public, but the module is private.

use crate::contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, UpdateCtx};
use crate::core::{RootState, WidgetState};

pub trait WidgetStateRef {
    fn widget_state(&self) -> &WidgetState;
}

pub trait WidgetStateMut: WidgetStateRef {
    fn widget_state_mut(&mut self) -> &mut WidgetState;
}

pub trait RootStateRef<'a> {
    fn root_state(&self) -> &RootState<'a>;
}

pub trait RootStateMut<'a>: RootStateRef<'a> {
    fn root_state_mut(&mut self) -> &mut RootState<'a>;
}

impl WidgetStateRef for EventCtx<'_, '_> {
    #[inline]
    fn widget_state(&self) -> &WidgetState {
        &self.widget_state
    }
}
impl WidgetStateMut for EventCtx<'_, '_> {
    #[inline]
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.widget_state
    }
}

impl WidgetStateRef for LifeCycleCtx<'_, '_> {
    #[inline]
    fn widget_state(&self) -> &WidgetState {
        &self.widget_state
    }
}
impl WidgetStateMut for LifeCycleCtx<'_, '_> {
    #[inline]
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.widget_state
    }
}

impl WidgetStateRef for UpdateCtx<'_, '_> {
    #[inline]
    fn widget_state(&self) -> &WidgetState {
        &self.widget_state
    }
}
impl WidgetStateMut for UpdateCtx<'_, '_> {
    #[inline]
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.widget_state
    }
}

impl WidgetStateRef for LayoutCtx<'_, '_, '_> {
    #[inline]
    fn widget_state(&self) -> &WidgetState {
        &self.widget_state
    }
}
impl WidgetStateMut for LayoutCtx<'_, '_, '_> {
    #[inline]
    fn widget_state_mut(&mut self) -> &mut WidgetState {
        &mut self.widget_state
    }
}

impl WidgetStateRef for PaintCtx<'_, '_> {
    #[inline]
    fn widget_state(&self) -> &WidgetState {
        &self.widget_state
    }
}

impl<'a> RootStateRef<'a> for EventCtx<'_, 'a> {
    #[inline]
    fn root_state(&self) -> &RootState<'a> {
        &self.root_state
    }
}
impl<'a> RootStateMut<'a> for EventCtx<'_, 'a> {
    #[inline]
    fn root_state_mut(&mut self) -> &mut RootState<'a> {
        &mut self.root_state
    }
}

impl<'a> RootStateRef<'a> for LifeCycleCtx<'_, 'a> {
    #[inline]
    fn root_state(&self) -> &RootState<'a> {
        &self.root_state
    }
}
impl<'a> RootStateMut<'a> for LifeCycleCtx<'_, 'a> {
    #[inline]
    fn root_state_mut(&mut self) -> &mut RootState<'a> {
        &mut self.root_state
    }
}

impl<'a> RootStateRef<'a> for UpdateCtx<'_, 'a> {
    #[inline]
    fn root_state(&self) -> &RootState<'a> {
        &self.root_state
    }
}
impl<'a> RootStateMut<'a> for UpdateCtx<'_, 'a> {
    #[inline]
    fn root_state_mut(&mut self) -> &mut RootState<'a> {
        &mut self.root_state
    }
}

impl<'a> RootStateRef<'a> for LayoutCtx<'_, '_, 'a> {
    #[inline]
    fn root_state(&self) -> &RootState<'a> {
        &self.root_state
    }
}
impl<'a> RootStateMut<'a> for LayoutCtx<'_, '_, 'a> {
    #[inline]
    fn root_state_mut(&mut self) -> &mut RootState<'a> {
        &mut self.root_state
    }
}
