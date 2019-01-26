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

//! Graph for structure for widget tree.

use std::mem;

use Id;

#[derive(Default)]
pub struct Graph {
    pub root: Id,
    pub children: Vec<Vec<Id>>,
    pub parent: Vec<Id>,

    free_list: Vec<Id>,
}

impl Graph {
    /// Allocate a node; it might be a previously freed id.
    pub fn alloc_node(&mut self) -> Id {
        if let Some(id) = self.free_list.pop() {
            return id;
        }
        let id = self.children.len();
        self.children.push(vec![]);
        self.parent.push(id);
        id
    }

    pub fn append_child(&mut self, parent: Id, child: Id) {
        self.children[parent].push(child);
        self.parent[child] = parent;
    }

    pub fn add_before(&mut self, parent: Id, sibling: Id, child: Id) {
        let pos = self.children[parent]
            .iter()
            .position(|&x| x == sibling)
            .expect("tried add_before nonexistent sibling");
        self.children[parent].insert(pos, child);
        self.parent[child] = parent;
    }

    /// Remove the child from the parent.
    ///
    /// Can panic if the graph structure is invalid. This function leaves the
    /// child in an unparented state, i.e. it can be added again.
    pub fn remove_child(&mut self, parent: Id, child: Id) {
        let ix = self.children[parent]
            .iter()
            .position(|&x| x == child)
            .expect("tried to remove nonexistent child");
        self.children[parent].remove(ix);
        self.parent[child] = child;
    }

    pub fn free_subtree(&mut self, node: Id) {
        let mut ix = self.free_list.len();
        // This is a little tricky; we're using the free list as a queue
        // for breadth-first traversal.
        self.free_list.push(node);
        while ix < self.free_list.len() {
            let node = self.free_list[ix];
            ix += 1;
            self.parent[node] = node;
            self.free_list
                .extend(mem::replace(&mut self.children[node], Vec::new()));
        }
    }
}
