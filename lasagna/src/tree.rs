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

use std::{any::Any, collections::HashMap, num::NonZeroU64};

#[derive(Default)]
pub struct Mutation {
    pub cmds: Option<Box<dyn Any>>,
    pub child: Vec<MutationEl>,
}

pub enum MutationEl {
    Skip(usize),
    // Note: the "dyn Any" here is not awesome, and will need
    // double boxing if the actual type is also dyn Trait, but
    // not gonna stress.
    Insert(Id, Box<dyn Any>, Mutation),
    Update(Mutation),
    Delete(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Hash)]
/// A stable identifier for an element.
pub struct Id(NonZeroU64);

/// The pure structure of a tree.
#[derive(Default)]
pub struct TreeStructure {
    parent: HashMap<Id, Option<Id>>,
    children: HashMap<Option<Id>, Vec<Id>>,
}

impl Id {
    /// Allocate a new, unique `Id`.
    pub fn next() -> Id {
        use druid_shell::Counter;
        static WIDGET_ID_COUNTER: Counter = Counter::new();
        Id(WIDGET_ID_COUNTER.next_nonzero())
    }

    pub fn to_raw(self) -> u64 {
        self.0.into()
    }
}

impl TreeStructure {
    pub fn parent(&self, id: Id) -> Option<Id> {
        None
    }

    pub fn children(&self, id: Option<Id>) -> Option<&[Id]> {
        None
    }

    pub fn apply(&mut self, mutation: &Mutation) {
        self.apply_rec(None, mutation);
    }

    fn apply_rec(&mut self, id: Option<Id>, mutation: &Mutation) {
        let mut i = 0;
        for el in &mutation.child {
            match el {
                MutationEl::Skip(n) => i += n,
                MutationEl::Insert(child_id, _, child_mut) => {
                    self.children.get_mut(&id).unwrap().insert(i, *child_id);
                    self.parent.insert(*child_id, id);
                    self.children.insert(Some(*child_id), Vec::new());
                    self.apply_rec(Some(*child_id), child_mut);
                    i += 1;
                }
                MutationEl::Update(child_mut) => {
                    let child_id = self.children.get(&id).unwrap()[i];
                    self.apply_rec(Some(child_id), child_mut);
                    i += 1;
                }
                MutationEl::Delete(n) => {
                    for j in i..i + n {
                        let child_id = self.children.get(&id).unwrap()[j];
                        self.delete(child_id);
                    }
                    self.children.get_mut(&id).unwrap().drain(i..i + n);
                }
            }
        }
    }

    fn delete(&mut self, id: Id) {
        self.parent.remove(&id);
        for child_id in self.children.remove(&Some(id)).unwrap() {
            self.delete(child_id);
        }
    }
}
