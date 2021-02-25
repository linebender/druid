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

use std::any::Any;

use druid_shell::{kurbo::Point, piet::Piet};

/// The trait for elements.
///
/// An interesting question for discussion: should this trait be integrated
/// with (widget-like) render objects, or should it be separate. One use case
/// for the latter is menus and other things that are not quite like widgets.
pub trait Element {
    fn mutate(&mut self, mutation: Mutation);

    fn paint(&mut self, ctx: &mut Piet, pos: Point);
}

pub struct Mutation {
    child: Vec<MutationEl>,
    // Should be a Vec? Option?
    // Deeper question: always apply after child mutation?
    cmds: Box<dyn Any>,
}

pub enum MutationEl {
    Skip(usize),
    // Id here?
    Insert(Box<dyn Element>, Mutation),
    Update(Mutation),
    Delete(usize),
}

pub struct Id(usize);
