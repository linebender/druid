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

use std::collections::HashMap;

use crate::element::{Action, Button, ButtonCmd, Column, Element};
use crate::tree::{Id, Mutation, MutationEl};

pub enum VdomNode<T> {
    /// A placeholder to maintain object identity when using
    /// conditional logic.
    Nil,
    Column(Vec<VdomNode<T>>),
    Button(String, Box<dyn FnMut(&mut T)>),
}

pub struct Reconciler<T> {
    callbacks: Cbks<T>,
    old_tree: OldNode,
}

type Cbks<T> = HashMap<Id, Box<dyn FnMut(&mut T)>>;

struct OldNode {
    id: Id,
    body: OldBody,
}

enum OldBody {
    Column(Vec<Option<OldNode>>),
    Button(String),
}

impl<T> Reconciler<T> {
    pub fn new(root_id: Id) -> Reconciler<T> {
        Reconciler {
            callbacks: Default::default(),
            old_tree: OldNode {
                id: root_id,
                body: OldBody::Column(Vec::new()),
            },
        }
    }

    pub fn run_actions(&mut self, actions: Vec<Action>, data: &mut T) {
        for action in actions {
            if let Some(callback) = self.callbacks.get_mut(&action.id) {
                callback(data);
            }
        }
    }

    pub fn reconcile(&mut self, vdom: VdomNode<T>) -> Mutation {
        if let VdomNode::Column(v) = vdom {
            if let OldBody::Column(old_vec) = &mut self.old_tree.body {
                Self::reconcile_vec(old_vec, v, &mut self.callbacks)
            } else {
                panic!("expected column at root");
            }
        } else {
            println!("oops, expecting a column as root node");
            Mutation::default()
        }
    }

    fn reconcile_vec(
        old_vec: &mut Vec<Option<OldNode>>,
        vdom_vec: Vec<VdomNode<T>>,
        cbks: &mut Cbks<T>,
    ) -> Mutation {
        let mut child = Vec::new();
        let n = vdom_vec.len();
        for (i, node) in vdom_vec.into_iter().enumerate() {
            if let Some(old_node) = old_vec.get_mut(i) {
                Self::reconcile_node(old_node, node, &mut child, cbks);
            } else {
                let mut old_node = None;
                Self::reconcile_node(&mut old_node, node, &mut child, cbks);
                old_vec.push(old_node);
            }
        }
        for mut old_node in old_vec.drain(n..) {
            Self::reconcile_node(&mut old_node, VdomNode::Nil, &mut child, cbks);
        }
        Mutation { cmds: None, child }
    }

    fn reconcile_node(
        old_node: &mut Option<OldNode>,
        vdom_node: VdomNode<T>,
        child: &mut Vec<MutationEl>,
        cbks: &mut Cbks<T>,
    ) {
        match vdom_node {
            VdomNode::Nil => {
                if let Some(old) = old_node {
                    child.push(MutationEl::Delete(1));
                    cbks.remove(&old.id);
                    *old_node = None;
                }
            }
            VdomNode::Button(s, cb) => {
                if let Some(OldNode {
                    body: OldBody::Button(old_s),
                    id,
                }) = old_node
                {
                    if s == *old_s {
                        child.push(MutationEl::Skip(1));
                    } else {
                        let cmds = ButtonCmd::SetText(s.clone());
                        let mutation = Mutation {
                            cmds: Some(Box::new(cmds)),
                            child: Vec::new(),
                        };
                        child.push(MutationEl::Update(mutation));
                        *old_s = s;
                    }
                    cbks.insert(*id, cb);
                } else {
                    if let Some(old) = old_node {
                        child.push(MutationEl::Delete(1));
                        cbks.remove(&old.id);
                    }
                    let id = Id::next();
                    let button = Button::default();
                    let dyn_button: Box<dyn Element> = Box::new(button);
                    // Note: we could create the button with the string rather than
                    // sending a mutation, but this way is likely easier to keep
                    // consistent.
                    let cmds = ButtonCmd::SetText(s.clone());
                    let mutation = Mutation {
                        cmds: Some(Box::new(cmds)),
                        child: Vec::new(),
                    };
                    *old_node = Some(OldNode {
                        id,
                        body: OldBody::Button(s),
                    });
                    child.push(MutationEl::Insert(id, Box::new(dyn_button), mutation));
                    cbks.insert(id, cb);
                }
            }
            VdomNode::Column(vdom_vec) => {
                if let Some(OldNode {
                    body: OldBody::Column(old_vec),
                    ..
                }) = old_node
                {
                    let mutation = Self::reconcile_vec(old_vec, vdom_vec, cbks);
                    child.push(MutationEl::Update(mutation));
                } else {
                    if let Some(old) = old_node {
                        child.push(MutationEl::Delete(1));
                        cbks.remove(&old.id);
                    }
                    let id = Id::next();
                    let column = Column::default();
                    let dyn_column: Box<dyn Element> = Box::new(column);
                    let mut old_vec = Vec::new();
                    let mutation = Self::reconcile_vec(&mut old_vec, vdom_vec, cbks);
                    *old_node = Some(OldNode {
                        id,
                        body: OldBody::Column(old_vec),
                    });
                    child.push(MutationEl::Insert(id, Box::new(dyn_column), mutation));
                }
            }
        }
    }
}
