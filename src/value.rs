// Copyright 2019 The xi-editor Authors.
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

//! The data model for application state.

use std::ops::Deref;
use std::sync::Arc;

use std::collections::HashMap;

/// This is a placeholder for a proper error type.
pub type Error = ();

/// The basic data type manipulated by this library.
///
/// This is similar to serde_json's `Value`, but with some important
/// differences. The main one is that it's reference counted for
/// low-cost sharing of subtrees.
#[derive(Clone, Debug)]
pub struct Value(Arc<ValueEnum>);

#[derive(Clone, Debug)]
pub enum ValueEnum {
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    // TODO: decide which of missing json values to include here (null, bool)
    // TODO: decide other things we might want to include (other num types, typed enum, binary)
}

/// One step in a path to identify a particular data element.
#[derive(Clone, Debug)]
pub enum PathEl {
    List(usize),
    Map(String),
}

// TODO: we probably want &[PathEl] for most read-only cases.
// Actually there are a lot of tradeoffs, it could be ref-counted and even a linked list
// to make lifetimes / cloning easier.
// Should be a newtype so we can have methods like join?
pub type KeyPath = Vec<PathEl>;

pub trait PathFragment {
    fn len(&self) -> usize;

    fn push_to_path(&self, path: &mut KeyPath);

    fn prepend_to_path(&self, path: &mut KeyPath) {
        self.push_to_path(path);
        path.rotate_right(self.len());
    }

    // Maybe do this as `From<> for KeyPath`? But that might cause coherence problems.
    // Also, this could be `self` but that would require a `Sized` bound.
    fn into_key_path(&self) -> KeyPath {
        let mut path = Vec::new();
        self.push_to_path(&mut path);
        path
    }

    // Maybe add value access methods here to cut down on allocating paths,
    // but that's just a performance optimization; could also be done with
    // an iterator.
}

/// One element of a delta - replaces a subtree at a specific location.
#[derive(Clone, Debug)]
pub struct DeltaEl {
    pub path: KeyPath,
    pub new_value: Option<Value>,
}

pub type Delta = Vec<DeltaEl>;

impl Default for Value {
    fn default() -> Value {
        // A good case can be made for a Null variant, and that it should be default.
        HashMap::new().into()
    }
}

impl Value {
    /// Get a reference to a subtree.
    pub fn access_by_path(&self, path: &KeyPath) -> Option<&Value> {
        let mut node = self;
        for el in path {
            match el {
                PathEl::List(ix) => {
                    if let ValueEnum::List(l) = node.0.deref() {
                        node = l.get(*ix)?;
                    } else {
                        return None;
                    }
                }
                PathEl::Map(key) => {
                    if let ValueEnum::Map(m) = node.0.deref() {
                        node = m.get(key)?;
                    } else {
                        return None;
                    }
                }
            }
        }
        Some(node)
    }

    pub fn access(&self, frag: impl PathFragment) -> Option<&Value> {
        // TODO: this could be optimized more (not allocating the intermediate path)
        self.access_by_path(&frag.into_key_path())
    }

    /// Apply a delta, resulting in a new value.
    ///
    /// Maybe supply the delta as impl `AsRef`?
    pub fn apply(&self, delta: &[DeltaEl]) -> Result<Value, Error> {
        let mut result = self.clone();
        for el in delta {
            result = result.apply_delta_el(el)?;
        }
        Ok(result)
    }

    fn apply_delta_el(&self, delta_el: &DeltaEl) -> Result<Value, Error> {
        self.apply_delta_rec(&delta_el.path, delta_el.new_value.as_ref())
    }

    fn apply_delta_rec(&self, path: &KeyPath, new_value: Option<&Value>) -> Result<Value, Error> {
        if let Some(el) = path.first() {
            match el {
                PathEl::List(ix) => {
                    if let ValueEnum::List(l) = self.0.deref() {
                        let mut l = l.clone();
                        if *ix < l.len() {
                            if let Some(new) = new_value {
                                l[*ix] = new.clone();
                            } else {
                                l.remove(*ix);
                            }
                        } else if *ix == l.len() {
                            if let Some(new) = new_value {
                                l.push(new.clone());
                            } else {
                                // We could choose to be more forgiving about deleting nonexisting elements
                                // but for now we'll basically declare that to be a logic error.
                                return Err(());
                            }
                        } else {
                            return Err(());
                        }
                        Ok(l.into())
                    } else {
                        Err(())
                    }
                }
                PathEl::Map(key) => {
                    if let ValueEnum::Map(m) = self.0.deref() {
                        let mut m = m.clone();
                        if let Some(new) = new_value {
                            m.insert(key.clone(), new.clone());
                        } else {
                            // Perhaps we should be stricter about deleting nonexisting elements; in any
                            // case more consistent with the array case.
                            m.remove(key);
                        }
                        Ok(m.into())
                    } else {
                        Err(())
                    }
                }
            }
        } else {
            Ok(self.clone())
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        if let ValueEnum::List(l) = self.0.deref() {
            Some(l)
        } else {
            None
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        if let ValueEnum::Map(m) = self.0.deref() {
            Some(m)
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let ValueEnum::Float(f) = self.0.deref() {
            Some(*f)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let ValueEnum::String(s) = self.0.deref() {
            Some(s)
        } else {
            None
        }
    }

    pub fn empty_map() -> Value {
        HashMap::new().into()
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value(Arc::new(ValueEnum::String(s)))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value(Arc::new(ValueEnum::String(s.into())))
    }
}

// add Cow<str> etc?

impl From<f64> for Value {
    fn from(f: f64) -> Value {
        Value(Arc::new(ValueEnum::Float(f)))
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Value {
        Value(Arc::new(ValueEnum::List(v)))
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Value {
        Value(Arc::new(ValueEnum::Map(m)))
    }
}

impl From<String> for PathEl {
    fn from(s: String) -> PathEl {
        PathEl::Map(s)
    }
}
impl From<usize> for PathEl {
    fn from(ix: usize) -> PathEl {
        PathEl::List(ix)
    }
}

impl PathFragment for () {
    fn len(&self) -> usize {
        0
    }

    fn push_to_path(&self, _path: &mut KeyPath) {}

    fn prepend_to_path(&self, _path: &mut KeyPath) {}
}

impl<'a> PathFragment for &'a str {
    fn len(&self) -> usize {
        1
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        path.push(self.to_string().into())
    }
}

impl PathFragment for usize {
    fn len(&self) -> usize {
        1
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        path.push((*self).into())
    }
}

impl<P0: PathFragment, P1: PathFragment> PathFragment for (P0, P1) {
    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        self.0.push_to_path(path);
        self.1.push_to_path(path);
    }
}

// TODO: larger tuples

impl PathFragment for KeyPath {
    fn len(&self) -> usize {
        self.len()
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        path.extend(self.iter().cloned());
    }
}

impl<'a> PathFragment for &KeyPath {
    fn len(&self) -> usize {
        (*self).len()
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        path.extend(self.iter().cloned());
    }
}

impl<'a> PathFragment for &'a [PathEl] {
    fn len(&self) -> usize {
        (*self).len()
    }

    fn push_to_path(&self, path: &mut KeyPath) {
        path.extend(self.iter().cloned());
    }
}
