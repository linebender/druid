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

//! A new architecture for app logic

use std::any::Any;
use std::marker::PhantomData;

use crate::element::{self, Action, ButtonCmd, Element};
use crate::tree::{Id, Mutation, MutationEl, TreeStructure};

pub struct RustyApp<T, V: View<T, ()>, F: FnMut(&mut T) -> V> {
    data: T,
    app: F,
    old_tree: Option<OldNode>,
    view: Option<V>,
    structure: TreeStructure,
    root_id: Id,
}

pub trait View<T, A> {
    // Likely type changes:
    // * OldNode will be an associated type
    // * previous: Option<Self> will be provided for diff reference
    fn reconcile(&self, old_node: &mut Option<OldNode>, child_mut: &mut Vec<MutationEl>);

    // Some type changes to consider:
    // * maybe mut self?
    // * old_node is mut?
    // * Always return A?
    fn event(
        &self,
        old_node: &OldNode,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A>;
}

pub struct OldNode {
    id: Id,
    body: Box<dyn Any>,
}

impl OldNode {
    fn downcast<T: 'static>(&self) -> Option<(Id, &T)> {
        let id = self.id;
        self.body.downcast_ref().map(|body| (id, body))
    }

    fn downcast_mut<T: 'static>(&mut self) -> Option<(Id, &mut T)> {
        let id = self.id;
        self.body.downcast_mut().map(|body| (id, body))
    }
}

pub struct Button<T, A> {
    text: String,
    // Callback might become a generic type parameter too.
    callback: Box<dyn Fn(ButtonAction, &mut T) -> A>,
}

pub struct ButtonAction;

struct ButtonOld {
    text: String,
}

pub struct Column<T, A> {
    children: Vec<Box<dyn View<T, A>>>,
}

#[derive(Default)]
struct ColumnOld {
    children: Vec<Option<OldNode>>,
}

// TODO: better name. "Map" is borrowed from elm.
pub struct Map<U, B, F, C> {
    f: F,
    child: C,
    // probably better to phantom the fn, but this shuts up the compiler
    phantom_u: PhantomData<U>,
    phantom_b: PhantomData<B>,
}

pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}

struct MemoizeOld<D> {
    data: D,
    child: Option<OldNode>,
}

impl<U, B, F, C: View<U, B>> Map<U, B, F, C> {
    fn new(f: F, child: C) -> Self {
        Map {
            f,
            child,
            phantom_u: Default::default(),
            phantom_b: Default::default(),
        }
    }
}

impl<T, A, U, B, F: Fn(&mut T, &dyn FnOnce(&mut U) -> Option<B>) -> A, C: View<U, B>> View<T, A>
    for Map<U, B, F, C>
{
    fn reconcile(&self, old_node: &mut Option<OldNode>, child_mut: &mut Vec<MutationEl>) {
        self.child.reconcile(old_node, child_mut);
    }

    fn event(
        &self,
        old_node: &OldNode,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        let a = (self.f)(app_state, &|u| {
            self.child.event(old_node, id_path, event_body, u)
        });
        Some(a)
    }
}

impl<T, V: View<T, ()>, F: FnMut(&mut T) -> V> RustyApp<T, V, F> {
    pub fn new(data: T, app: F) -> Self {
        // This is bogus, and possibly should be changed to be the
        // actual id of the root element, or there should be refactoring
        // so it's not needed.
        let dummy_id = Id::next();
        let old_tree = Some(OldNode {
            id: dummy_id,
            body: Box::new(ColumnOld::default()),
        });
        RustyApp {
            data,
            app,
            old_tree,
            view: None,
            structure: TreeStructure::new(),
            root_id: dummy_id,
        }
    }

    pub fn run(&mut self, actions: Vec<Action>) -> Mutation {
        if let (Some(view), Some(old_tree)) = (&self.view, &self.old_tree) {
            let mut id_path = Vec::new();
            for action in actions {
                // get id path from structure
                id_path.clear();
                let mut id = Some(action.id);
                while let Some(this_id) = id {
                    id_path.push(this_id);
                    id = self.structure.parent(this_id).flatten();
                }
                // This is a hack; rethink
                id_path.push(self.root_id);
                id_path.reverse();
                view.event(old_tree, &id_path, action.action, &mut self.data);
            }
        }
        let view = (self.app)(&mut self.data);
        let mut child_mut = Vec::new();
        view.reconcile(&mut self.old_tree, &mut child_mut);
        self.view = Some(view);
        let mut_el = child_mut.pop().expect("empty root mutation");
        if let MutationEl::Update(mutation) = mut_el {
            self.structure.apply(&mutation);
            mutation
        } else {
            panic!("expected root mutation to be an update");
        }
    }
}

impl<T, A> Button<T, A> {
    pub fn new(
        text: impl Into<String>,
        callback: impl Fn(ButtonAction, &mut T) -> A + 'static,
    ) -> Self {
        Button {
            text: text.into(),
            callback: Box::new(callback),
        }
    }
}

impl<T, A> View<T, A> for Button<T, A> {
    fn reconcile(&self, old_node: &mut Option<OldNode>, child_mut: &mut Vec<MutationEl>) {
        if let Some((_id, ButtonOld { text })) = old_node.as_mut().and_then(OldNode::downcast_mut) {
            if self.text == *text {
                child_mut.push(MutationEl::Skip(1));
            } else {
                let cmds = ButtonCmd::SetText(self.text.clone());
                let mutation = Mutation {
                    cmds: Some(Box::new(cmds)),
                    child: Vec::new(),
                };
                child_mut.push(MutationEl::Update(mutation));
                *text = self.text.clone();
            }
        } else {
            if old_node.is_some() {
                child_mut.push(MutationEl::Delete(1));
            }
            let id = Id::next();
            let button = element::Button::default();
            let dyn_button: Box<dyn Element> = Box::new(button);
            // Note: we could create the button with the string rather than
            // sending a mutation, but this way is likely easier to keep
            // consistent.
            let cmds = ButtonCmd::SetText(self.text.clone());
            let mutation = Mutation {
                cmds: Some(Box::new(cmds)),
                child: Vec::new(),
            };
            *old_node = Some(OldNode {
                id,
                body: Box::new(ButtonOld {
                    text: self.text.clone(),
                }),
            });
            child_mut.push(MutationEl::Insert(id, Box::new(dyn_button), mutation));
        }
    }

    fn event(
        &self,
        old_node: &OldNode,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        if let Some(button_event) = event_body.downcast_ref::<()>() {
            assert_eq!(old_node.id, id_path[0]);
            Some((self.callback)(ButtonAction, app_state))
        } else {
            None
        }
    }
}

fn reconcile_vec<T, A>(
    old_vec: &mut Vec<Option<OldNode>>,
    view_vec: &[Box<dyn View<T, A>>],
) -> Mutation {
    let mut child = Vec::new();
    //let n = view_vec.len();
    for (i, view) in view_vec.iter().enumerate() {
        if let Some(old_node) = old_vec.get_mut(i) {
            view.reconcile(old_node, &mut child);
        } else {
            let mut old_node = None;
            view.reconcile(&mut old_node, &mut child);
            old_vec.push(old_node);
        }
    }
    // TODO: delete n..
    Mutation { cmds: None, child }
}

impl<T, A> Column<T, A> {
    pub fn new() -> Self {
        Column {
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: impl View<T, A> + 'static) {
        self.children.push(Box::new(child));
    }
}

impl<T, A> View<T, A> for Column<T, A> {
    fn reconcile(&self, old_node: &mut Option<OldNode>, child_mut: &mut Vec<MutationEl>) {
        if let Some((_id, ColumnOld { children })) =
            old_node.as_mut().and_then(OldNode::downcast_mut)
        {
            let mutation = reconcile_vec(children, &self.children);
            child_mut.push(MutationEl::Update(mutation));
        } else {
            // TODO: handle insert case
        }
    }

    fn event(
        &self,
        old_node: &OldNode,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        if let Some((_id, ColumnOld { children })) = old_node.downcast() {
            // check id_path[0] == old_node.id?
            let id = id_path[1];
            for (i, node) in children.iter().enumerate() {
                if let Some(node) = node {
                    if node.id == id {
                        return self.children[i].event(node, &id_path[1..], event_body, app_state);
                    }
                }
            }
            println!("event id not found in children");
        }
        None
    }
}

impl<D, V, F: Fn(&D) -> V> Memoize<D, F> {
    pub fn new(data: D, child_cb: F) -> Self {
        Memoize { data, child_cb }
    }
}

impl<T, A, D: PartialEq + Clone + 'static, V: View<T, A>, F: Fn(&D) -> V> View<T, A>
    for Memoize<D, F>
{
    fn reconcile(&self, old_node: &mut Option<OldNode>, child_mut: &mut Vec<MutationEl>) {
        if let Some((_id, old)) = old_node
            .as_mut()
            .and_then(OldNode::downcast_mut::<MemoizeOld<D>>)
        {
            if old.data == self.data {
                child_mut.push(MutationEl::Skip(1));
            } else {
                old.data = self.data.clone();
                let view = (self.child_cb)(&self.data);
                view.reconcile(&mut old.child, child_mut);
            }
        } else {
            if old_node.is_some() {
                child_mut.push(MutationEl::Delete(1));
            }
            let mut memo_old = MemoizeOld {
                data: self.data.clone(),
                child: None,
            };
            let view = (self.child_cb)(&self.data);
            view.reconcile(&mut memo_old.child, child_mut);
            let id = if let Some(child_old) = &memo_old.child {
                child_old.id
            } else {
                panic!("memoize child didn't create an id");
            };
            *old_node = Some(OldNode {
                id,
                body: Box::new(memo_old),
            });
        }
    }

    fn event(
        &self,
        old_node: &OldNode,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        let view = (self.child_cb)(&self.data);
        if let Some((_id, child)) = old_node.downcast::<MemoizeOld<D>>() {
            view.event(
                child.child.as_ref().unwrap(),
                id_path,
                event_body,
                app_state,
            )
        } else {
            println!("memoize downcast failed");
            None
        }
    }
}
