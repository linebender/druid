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

//! An Elm-like architecture for writing app logic.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::element::{self, Action, ButtonCmd, Element};
use crate::tree::{Id, Mutation, MutationEl};

pub struct ElmApp<A: AppLogic> {
    app_logic: A,
    action_map: HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> A::Msg>>,
    old_tree: Option<OldNode>,
}

pub trait AppLogic {
    type Msg;
    fn update(&mut self, msg: Self::Msg);
    fn view(&mut self) -> Box<dyn Vdom<Self::Msg>>;
}

/// A trait representing a vdom node.
///
/// Here, `M` is the type of the message produced by this subtree of the
/// vdom. It is also parametrized on `R`, which is the message type at the
/// root of the vdom tree.
///
/// It would be possible to eliminate the `R` parameter by using `Any`
/// instead, and there might also be a way to do it with no additional
/// performance cost. (Making the `reconcile` method itself be parameterized
/// would destroy object safety.)
pub trait Vdom<M, R = M> {
    fn reconcile(
        self: Box<Self>,
        mapper: &Arc<dyn Fn(M) -> R>,
        action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
        old_node: &mut Option<OldNode>,
        child_mut: &mut Vec<MutationEl>,
    );
}

pub struct OldNode {
    id: Id,
    body: Box<dyn Any>,
}

impl OldNode {
    fn downcast<T: 'static>(&mut self) -> Option<(Id, &mut T)> {
        let id = self.id;
        self.body.downcast_mut().map(|body| (id, body))
    }
}

pub struct Button<M> {
    text: String,
    mapper: Box<dyn FnMut(ButtonAction) -> M>,
}

/// The action for a button (click).
///
/// This should probably be in the element.
pub struct ButtonAction;

struct ButtonOld {
    text: String,
}

pub struct Column<M, R> {
    children: Vec<Box<dyn Vdom<M, R>>>,
}

#[derive(Default)]
struct ColumnOld {
    children: Vec<Option<OldNode>>,
}

pub struct Map<M, N, V>(Box<V>, Box<dyn Fn(N) -> M>);

impl<A: AppLogic> ElmApp<A> {
    pub fn new(app_logic: A) -> ElmApp<A> {
        // This is bogus, and possibly should be changed to be the
        // actual id of the root element, or there should be refactoring
        // so it's not needed.
        let dummy_id = Id::next();
        let old_tree = Some(OldNode {
            id: dummy_id,
            body: Box::new(ColumnOld::default()),
        });
        ElmApp {
            app_logic,
            action_map: HashMap::new(),
            old_tree,
        }
    }

    pub fn run(&mut self, actions: Vec<Action>) -> Mutation {
        for action in actions {
            if let Some(mapper) = self.action_map.get_mut(&action.id) {
                let msg = mapper(action.action);
                self.app_logic.update(msg);
            }
        }
        let vdom = self.app_logic.view();
        let mut child_mut = Vec::new();
        let mapper: Arc<dyn Fn(A::Msg) -> A::Msg> = Arc::new(|a| a);
        vdom.reconcile(
            &mapper,
            &mut self.action_map,
            &mut self.old_tree,
            &mut child_mut,
        );
        let mut_el = child_mut.pop().expect("empty root mutation");
        if let MutationEl::Update(mutation) = mut_el {
            mutation
        } else {
            panic!("expected root mutation to be an update");
        }
    }
}

impl<M> Button<M> {
    pub fn new(text: impl Into<String>, mapper: impl FnMut(ButtonAction) -> M + 'static) -> Self {
        Button {
            text: text.into(),
            mapper: Box::new(mapper),
        }
    }
}

impl<M: 'static, R: 'static> Vdom<M, R> for Button<M> {
    fn reconcile(
        self: Box<Self>,
        mapper: &Arc<dyn Fn(M) -> R>,
        action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
        old_node: &mut Option<OldNode>,
        child_mut: &mut Vec<MutationEl>,
    ) {
        if let Some((id, ButtonOld { text })) = old_node.as_mut().and_then(OldNode::downcast) {
            if self.text == *text {
                child_mut.push(MutationEl::Skip(1));
            } else {
                let cmds = ButtonCmd::SetText(self.text.clone());
                let mutation = Mutation {
                    cmds: Some(Box::new(cmds)),
                    child: Vec::new(),
                };
                child_mut.push(MutationEl::Update(mutation));
                *text = self.text;
            }
            let mapper = mapper.clone();
            let mut self_mapper = self.mapper;
            let f = move |_action| (mapper)((self_mapper)(ButtonAction));
            action_map.insert(id, Box::new(f));
        } else {
            if let Some(old) = old_node {
                child_mut.push(MutationEl::Delete(1));
                action_map.remove(&old.id);
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
                body: Box::new(ButtonOld { text: self.text }),
            });
            child_mut.push(MutationEl::Insert(id, Box::new(dyn_button), mutation));
            let mapper = mapper.clone();
            let mut self_mapper = self.mapper;
            let f = move |_action| (mapper)((self_mapper)(ButtonAction));
            action_map.insert(id, Box::new(f));
        }
    }
}

fn reconcile_nil<R>(
    action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
    old_node: &mut Option<OldNode>,
    child_mut: &mut Vec<MutationEl>,
) {
    if let Some(old) = old_node {
        child_mut.push(MutationEl::Delete(1));
        action_map.remove(&old.id);
        *old_node = None;
    }
}

/// We let the unit stand for "no element".
///
/// It might be a better idea to let vdom nodes be option, so that
/// these don't need to get boxed, but that makes types spammier.
impl<M: 'static, R: 'static> Vdom<M, R> for () {
    fn reconcile(
        self: Box<Self>,
        _mapper: &Arc<dyn Fn(M) -> R>,
        action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
        old_node: &mut Option<OldNode>,
        child_mut: &mut Vec<MutationEl>,
    ) {
        reconcile_nil(action_map, old_node, child_mut);
    }
}

fn reconcile_vec<M: 'static, R: 'static>(
    old_vec: &mut Vec<Option<OldNode>>,
    vdom_vec: Vec<Box<dyn Vdom<M, R>>>,
    mapper: &Arc<dyn Fn(M) -> R>,
    action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
) -> Mutation {
    let mut child = Vec::new();
    let n = vdom_vec.len();
    for (i, node) in vdom_vec.into_iter().enumerate() {
        if let Some(old_node) = old_vec.get_mut(i) {
            node.reconcile(mapper, action_map, old_node, &mut child);
        } else {
            let mut old_node = None;
            node.reconcile(mapper, action_map, &mut old_node, &mut child);
            old_vec.push(old_node);
        }
    }
    for mut old_node in old_vec.drain(n..) {
        reconcile_nil(action_map, &mut old_node, &mut child);
    }
    Mutation { cmds: None, child }
}

impl<M, R> Column<M, R> {
    pub fn new() -> Self {
        Column {
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: impl Vdom<M, R> + 'static) {
        self.children.push(Box::new(child));
    }
}

impl<M: 'static, R: 'static> Vdom<M, R> for Column<M, R> {
    fn reconcile(
        self: Box<Self>,
        mapper: &Arc<dyn Fn(M) -> R>,
        action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
        old_node: &mut Option<OldNode>,
        child_mut: &mut Vec<MutationEl>,
    ) {
        if let Some((_id, ColumnOld { children })) = old_node.as_mut().and_then(OldNode::downcast) {
            let mutation = reconcile_vec(children, self.children, mapper, action_map);
            child_mut.push(MutationEl::Update(mutation));
        } else {
            if let Some(old) = old_node {
                child_mut.push(MutationEl::Delete(1));
                action_map.remove(&old.id);
            }
            let id = Id::next();
            let column = element::Column::default();
            let dyn_column: Box<dyn Element> = Box::new(column);
            let mut old_vec = Vec::new();
            let mutation = reconcile_vec(&mut old_vec, self.children, mapper, action_map);
            *old_node = Some(OldNode {
                id,
                body: Box::new(ColumnOld { children: old_vec }),
            });
            child_mut.push(MutationEl::Insert(id, Box::new(dyn_column), mutation));
        }
    }
}

impl<M: 'static, N: 'static, R: 'static, V> Vdom<M, R> for Map<M, N, V>
where
    V: Vdom<N, R>,
{
    fn reconcile(
        self: Box<Self>,
        mapper: &Arc<dyn Fn(M) -> R>,
        action_map: &mut HashMap<Id, Box<dyn FnMut(Box<dyn Any>) -> R>>,
        old_node: &mut Option<OldNode>,
        child_mut: &mut Vec<MutationEl>,
    ) {
        let f = self.1;
        let mapper = mapper.clone();
        let child_mapper: Arc<dyn Fn(N) -> R> = Arc::new(move |a| mapper(f(a)));
        self.0
            .reconcile(&child_mapper, action_map, old_node, child_mut);
    }
}
