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
use std::ops::Deref;

use crate::element::{self, Action, ButtonCmd, Element};
use crate::tree::{ChildMutation, Id, Mutation, MutationEl, MutationFragment, TreeStructure};

pub struct RustyApp<T, F: FnMut(&mut T) -> Column<T, ()>> {
    data: T,
    app: F,
    view: Column<T, ()>,
    state: Option<ColumnState>,
    structure: TreeStructure,
    root_id: Option<Id>,
}

/// A view object representing a node in the UI.
///
/// This is the central trait for representing the UI; an app will generate
/// a tree of these objects, which in turn will render the UI and handle
/// event dispatch.
///
/// View objects are lightweight and transitory. Rendering proceeds by diffing
/// the view against the previous version; this basic pattern is common to
/// Elm, React, Flutter, and SwiftUI, among many others.
///
/// Views are parameterized by the "app state" and also an action (or "message"
/// in Elm lingo) type, which is passed up the tree in event propagation. These
/// types are used for event dispatch only, and are not needed for rendering.
/// It is possible to implement the pure Elm architecture by having `T` empty
/// except for one node at the top of the view tree, and relying exclusively
/// on `A`.
pub trait View<T, A> {
    /// The associated state for this view.
    ///
    /// Each view has an associated state object, which persists across renders.
    type State;

    /// Reconcile the view with the previous view, updating the element tree.
    ///
    /// On first render, `prev`, `id`, and `state` will be `None`. In that case,
    /// this method creates a node for the element tree, and adds it to the
    /// mutation as an `Insert` element. It also updates the `id` to the id of
    /// the newly created element and initializes its state object.
    ///
    /// Subsequently, the previous view node is made available for diffing,
    /// and there is mutable access to state.
    ///
    /// The main goal of this method is to produce a mutation corresponding to
    /// the subtree of the element tree corresponding to this node. There is
    /// flexibility regarding exactly how that mutation is produced. If there
    /// is no change since the last reconcile pass, it can simply add a `Skip`
    /// element. Or, the method may employ custom logic for creating the
    /// mutation. Of course, the usual approach for container views is to
    /// recurse to the children.
    // consider State: Default rather than option
    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<Self::State>,
    ) -> MutationFragment;

    /// Dispatch an event.
    ///
    /// An event is produced by the element tree as an id and an element
    /// body, which has a type corresponding to the node in the element
    /// tree.
    ///
    /// Dispatching is based on the id_path, which is a sequence of element
    /// ids from this node to the node in the element tree that produced the
    /// event. A container node is expected to retain the element ids of its
    /// children in its state, then recursively call this method on the child,
    /// trimming off one element from the id_path slice.
    ///
    /// Event processing has mutable access to the app state.
    // Some type changes to consider:
    // * maybe mut self?
    // * state is mut?
    // * Always return A?
    //   - maybe more ergonomic if A: Default?
    fn event(
        &self,
        state: &Self::State,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A>;
}

/// An internal trait for dynamic boxing of view objects.
///
/// Consider renaming to AnyView. Similar patterns exist in other crates (for
/// example, ErasedVirtualDom in Panoramix). I'm searching for good docs.
pub trait DynView<T, A> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_reconcile(
        &self,
        prev: Option<&dyn DynView<T, A>>,
        id: &mut Option<Id>,
        state: &mut Option<Box<dyn Any>>,
    ) -> MutationFragment;

    fn dyn_event(
        &self,
        state: &dyn Any,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A>;
}

impl<T, A, V: View<T, A> + 'static> DynView<T, A> for V
where
    V::State: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_reconcile(
        &self,
        prev: Option<&dyn DynView<T, A>>,
        id: &mut Option<Id>,
        state: &mut Option<Box<dyn Any>>,
    ) -> MutationFragment {
        let mut delete = false;
        if let Some(prev) = prev {
            if let Some(prev) = prev.as_any().downcast_ref() {
                if let Some(state) = state {
                    if let Some(state) = state.downcast_mut() {
                        return self.reconcile(Some(prev), id, state);
                    }
                }
            }
            delete = true;
            *id = None;
        }
        let mut child_state = None;
        let mut fragment = self.reconcile(None, id, &mut child_state);
        fragment.delete = delete;
        *state = Some(Box::new(child_state));
        fragment
    }

    fn dyn_event(
        &self,
        state: &dyn Any,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        if let Some(state) = state.downcast_ref::<Option<V::State>>() {
            self.event(state.as_ref().unwrap(), id_path, event_body, app_state)
        } else {
            println!("downcast error in event");
            None
        }
    }
}

impl<T: 'static, A: 'static> View<T, A> for Box<dyn DynView<T, A>> {
    type State = Box<dyn Any>;

    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<Self::State>,
    ) -> MutationFragment {
        self.deref()
            .dyn_reconcile(prev.map(|d| d.deref()), id, state)
    }

    fn event(
        &self,
        state: &Self::State,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        self.deref()
            .dyn_event(state.deref(), id_path, event_body, app_state)
    }
}

/// Your basic button.
///
/// This is currently very spare, but suffices to show setting of properties
/// (the text) and delivering events (button clicks).
pub struct Button<T, A> {
    text: String,
    // Callback might become a generic type parameter too.
    callback: Box<dyn Fn(ButtonAction, &mut T) -> A>,
}

pub struct ButtonAction;

/// A column of widgets.
///
/// This is currently implemented using dynamically typed (type-erased)
/// children, which is not where we're headed. For most uses, we will have a
/// `ViewTuple` trait, implemented by (V0, V1) etc, so that everything is
/// statically typed. The dynamic size case will be handled by a column of
/// widgets all of the same type.
///
/// However, the implementation does shed light on how to handle the
/// dynamic case.
pub struct Column<T, A> {
    children: Vec<Box<dyn DynView<T, A>>>,
}

#[derive(Default)]
pub struct ColumnState {
    children: Vec<(Option<Id>, Option<Box<dyn Any>>)>,
}

/// An adapter between different app state and action types.
///
/// This is very similar to, and a generalization of, the [Html map] method
/// in Elm. It also functions similarly to a lens (as in Druid).
///
/// The child can have a different app state type and action type, and the
/// supplied callback can apply arbitrary logic.
///
/// Unlike Elm, the callback has mutable access to the (parent) app state, so
/// it do things such as merge in changes made (by event processing of the
/// child view) to the child app state, set a dirty flag, etc.
///
/// [Html map]: https://package.elm-lang.org/packages/elm/html/latest/Html#map
// TODO: better name. "Map" is borrowed from elm.
pub struct Map<T, A, U, B, F: Fn(&mut T, MapThunk<U, B, C>) -> A, C: View<U, B>> {
    f: F,
    child: C,
    // probably better to phantom the fn, but this shuts up the compiler
    phantom_t: PhantomData<T>,
    phantom_a: PhantomData<A>,
    phantom_u: PhantomData<U>,
    phantom_b: PhantomData<B>,
}

/// A "thunk" which dispatches an event to a map's child.
///
/// The closure passed to the Map should call this thunk with the child's
/// app state.
pub struct MapThunk<'a, U, B, C: View<U, B>> {
    child: &'a C,
    state: &'a C::State,
    id_path: &'a [Id],
    event_body: Box<dyn Any>,
}

/// A memoize node.
///
/// This node is given some data (which implements `Clone` and `PartialEq`),
/// and skips re-rendering the child when that data has not changed. It always
/// renders on first view.
///
/// It is more or less identical to [Html lazy] in Elm. We also anticipate
/// similar nodes that will check pointer equality of `Rc`/`Arc`, and also a
/// `bool` dirty parameter, which is explicitly set by the app.
///
/// [Html lazy]: https://package.elm-lang.org/packages/elm/html/latest/Html-Lazy
pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}

pub struct MemoizeState<T, A, V: View<T, A>> {
    view: V,
    view_state: Option<V::State>,
}

impl<T, A, U, B, F: Fn(&mut T, MapThunk<U, B, C>) -> A, C: View<U, B>> Map<T, A, U, B, F, C> {
    pub fn new(f: F, child: C) -> Self {
        Map {
            f,
            child,
            phantom_t: Default::default(),
            phantom_a: Default::default(),
            phantom_u: Default::default(),
            phantom_b: Default::default(),
        }
    }
}

impl<'a, U, B, C: View<U, B>> MapThunk<'a, U, B, C> {
    pub fn call(self, app_state: &mut U) -> Option<B> {
        self.child
            .event(self.state, self.id_path, self.event_body, app_state)
    }
}

impl<T, A, U, B, F: Fn(&mut T, MapThunk<U, B, C>) -> A, C: View<U, B>> View<T, A>
    for Map<T, A, U, B, F, C>
{
    type State = C::State;

    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<C::State>,
    ) -> MutationFragment {
        self.child.reconcile(prev.map(|m| &m.child), id, state)
    }

    fn event(
        &self,
        state: &C::State,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        let thunk = MapThunk {
            child: &self.child,
            state,
            id_path,
            event_body,
        };
        let a = (self.f)(app_state, thunk);
        Some(a)
    }
}

impl<T, F: FnMut(&mut T) -> Column<T, ()>> RustyApp<T, F> {
    pub fn new(data: T, app: F) -> Self {
        // This is bogus, and possibly should be changed to be the
        // actual id of the root element, or there should be refactoring
        // so it's not needed.
        let dummy_id = Id::next();
        RustyApp {
            data,
            app,
            view: Column::new(),
            state: Some(ColumnState::default()),
            structure: TreeStructure::new(),
            root_id: Some(dummy_id),
        }
    }

    pub fn run(&mut self, actions: Vec<Action>) -> Mutation {
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
            id_path.push(self.root_id.unwrap());
            id_path.reverse();
            self.view.event(
                self.state.as_ref().unwrap(),
                &id_path,
                action.action,
                &mut self.data,
            );
        }
        let view = (self.app)(&mut self.data);
        let mut child_mut = ChildMutation::default();
        let frag = view.reconcile(Some(&self.view), &mut self.root_id, &mut self.state);
        child_mut.push(frag);
        self.view = view;
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
    type State = ();

    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<()>,
    ) -> MutationFragment {
        if let Some(prev) = prev {
            if self.text == prev.text {
                MutationEl::Skip(1).into()
            } else {
                let cmds = ButtonCmd::SetText(self.text.clone());
                let mutation = Mutation {
                    cmds: Some(Box::new(cmds)),
                    child: ChildMutation::default(),
                };
                MutationEl::Update(mutation).into()
            }
        } else {
            let new_id = Id::next();
            let button = element::Button::default();
            let dyn_button: Box<dyn Element> = Box::new(button);
            // Note: we could create the button with the string rather than
            // sending a mutation, but this way is likely easier to keep
            // consistent.
            let cmds = ButtonCmd::SetText(self.text.clone());
            let mutation = Mutation {
                cmds: Some(Box::new(cmds)),
                child: ChildMutation::default(),
            };
            *id = Some(new_id);
            *state = Some(());
            MutationEl::Insert(new_id, Box::new(dyn_button), mutation).into()
        }
    }

    fn event(
        &self,
        _state: &(),
        _id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        if let Some(_button_event) = event_body.downcast_ref::<()>() {
            Some((self.callback)(ButtonAction, app_state))
        } else {
            None
        }
    }
}

fn reconcile_vec<T, A>(
    prev: &[Box<dyn DynView<T, A>>],
    children: &[Box<dyn DynView<T, A>>],
    state: &mut Vec<(Option<Id>, Option<Box<dyn Any>>)>,
) -> ChildMutation {
    let mut child = ChildMutation::default();
    for (i, view) in children.iter().enumerate() {
        if let (Some(prev_child), Some((id, state))) = (prev.get(i), state.get_mut(i)) {
            child.push(
                view.deref()
                    .dyn_reconcile(Some(prev_child.deref()), id, state),
            );
        } else {
            let mut id = None;
            let mut child_state = None;
            child.push(view.deref().dyn_reconcile(None, &mut id, &mut child_state));
            state.push((id, child_state));
        }
    }
    if prev.len() > children.len() {
        state.truncate(children.len());
        child.push(MutationEl::Delete(prev.len() - children.len()));
    }
    child
}

impl<T, A> Column<T, A> {
    pub fn new() -> Self {
        Column {
            children: Vec::new(),
        }
    }

    pub fn add_child<V: View<T, A> + 'static>(&mut self, child: V)
    where
        V::State: 'static,
    {
        self.children.push(Box::new(child));
    }
}

impl<T, A> View<T, A> for Column<T, A> {
    type State = ColumnState;

    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<ColumnState>,
    ) -> MutationFragment {
        if let Some(prev) = prev {
            let child = reconcile_vec(
                &prev.children,
                &self.children,
                &mut state.as_mut().unwrap().children,
            );
            child.into_fragment()
        } else {
            let new_id = Id::next();
            let column = element::Column::default();
            let dyn_column: Box<dyn Element> = Box::new(column);
            let mut children = Vec::new();
            let child = reconcile_vec(&[], &self.children, &mut children);
            *id = Some(new_id);
            *state = Some(ColumnState { children });
            let mutation = Mutation { cmds: None, child };
            MutationEl::Insert(new_id, Box::new(dyn_column), mutation).into()
        }
    }

    fn event(
        &self,
        state: &ColumnState,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        let id = id_path[1];
        for (i, (child_id, child_state)) in state.children.iter().enumerate() {
            if *child_id == Some(id) {
                return self.children[i].deref().dyn_event(
                    child_state.as_ref().unwrap().deref(),
                    &id_path[1..],
                    event_body,
                    app_state,
                );
            }
        }
        println!("event id not found in children");
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
    type State = MemoizeState<T, A, V>;

    fn reconcile(
        &self,
        prev: Option<&Self>,
        id: &mut Option<Id>,
        state: &mut Option<Self::State>,
    ) -> MutationFragment {
        if let Some(prev) = prev {
            if prev.data == self.data {
                MutationEl::Skip(1).into()
            } else {
                let state = state.as_mut().unwrap();
                let view = (self.child_cb)(&self.data);
                let frag = view.reconcile(Some(&state.view), id, &mut state.view_state);
                state.view = view;
                frag
            }
        } else {
            let mut view_state = None;
            let view = (self.child_cb)(&self.data);
            let frag = view.reconcile(None, id, &mut view_state);
            *state = Some(MemoizeState { view, view_state });
            frag
        }
    }

    fn event(
        &self,
        state: &Self::State,
        id_path: &[Id],
        event_body: Box<dyn Any>,
        app_state: &mut T,
    ) -> Option<A> {
        state.view.event(
            state.view_state.as_ref().unwrap(),
            id_path,
            event_body,
            app_state,
        )
    }
}
