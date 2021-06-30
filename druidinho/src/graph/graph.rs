use super::value::{AttributeStore, Binding, DataId, ValueType};
use std::collections::HashSet;

pub struct RegistrationCtx {
    pub store: AttributeStore,
    pub tree: Vec<AttributeNode>,
}

impl Default for RegistrationCtx {
    fn default() -> Self {
        RegistrationCtx {
            store: Default::default(),
            tree: vec![AttributeNode::default()],
        }
    }
}

//pub trait AttributedThing {
    //fn register(&mut self, ctx: &mut RegistrationCtx);
//}

#[derive(Clone, Debug, Default)]
pub struct AttributeNode {
    children: Vec<AttributeNode>,
    data: HashSet<DataId>,
}

impl RegistrationCtx {
    pub fn start_child(&mut self) {
        self.tree.push(AttributeNode::default());
    }

    pub fn end_child(&mut self) {
        let child = self
            .tree
            .pop()
            .expect("*very* unbalanced child registration");
        let parent = self.tree.last_mut().expect("unbalanced child registration");
        parent.add_child(child);
    }

    /// Register a binding as the canonical store for this data.
    pub fn register<T: ValueType>(&mut self, binding: &Binding<T>) {
        self.tree.last_mut().unwrap().data.insert(binding.id());
        self.store.insert(binding);
    }

    pub fn bind<T: ValueType>(&mut self, binding: &mut Binding<T>) {
        self.tree.last_mut().unwrap().data.insert(binding.id());
        let value = self.store.get_value(binding).unwrap();
        binding.value = value;
    }
}

impl AttributeNode {
    fn add_child(&mut self, child: AttributeNode) {
        self.data.extend(child.data.iter().copied());
        self.children.push(child);
    }
}

pub struct MutationCtx<'a> {
    pub attributes: &'a mut AttributeStore,
}

pub struct UpdateCtx<'a> {
    pub attributes: &'a mut AttributeStore,
    stack: Vec<(&'a AttributeNode, usize)>,
    //child_idx: usize,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(attributes: &'a mut AttributeStore, root: &'a AttributeNode) -> Self {
        UpdateCtx {
            attributes,
            stack: vec![(root, 0)],
            //child_idx: 0,
        }
    }

    //TODO: would be nice if this returned a `bool` indicating if the
    //data data changed?
    pub fn update<T: ValueType>(&self, binding: &mut Binding<T>) {
        if !self.attributes.changes.contains(&binding.id()) {
            return;
        }

        let value = self
            .attributes
            .get_value(binding)
            .expect("invalid value type for binding");

        let tree_depth = self.stack.len() - 1;
        eprintln!(
            "{}updated value {:?} ({:?}->{:?})",
            &SPACES[..tree_depth * 2],
            binding.id(),
            &binding.value,
            &value
        );
        binding.value = value;
    }

    pub fn should_recurse(&self) -> bool {
        let should_recurse = self
            .stack
            .last()
            .map(|(node, _)| {
                self.attributes
                    .changes
                    .iter()
                    .any(|id| node.data.contains(id))
            })
            .unwrap();

        let tree_depth = self.stack.len() - 1;
        let indent = &SPACES[..tree_depth * 2];
        let text = if should_recurse { "recurse" } else { "skip" };
        eprintln!("{}{}", indent, text);
        should_recurse
    }

    pub fn start_child(&mut self) {
        let child = self
            .stack
            .last()
            .and_then(|(node, idx)| node.children.get(*idx))
            .unwrap();
        self.stack.push((child, 0));
    }

    pub fn end_child(&mut self) {
        self.stack.pop();
        self.stack.last_mut().unwrap().1 += 1;
    }

    pub fn tree_depth(&self) -> usize {
        self.stack.len() - 1
    }
}

static SPACES: &str = "                                                ";

impl MutationCtx<'_> {
    pub fn set<T: ValueType>(&mut self, binding: &Binding<T>, new: T) {
        self.attributes.set(binding, new)
    }
}
