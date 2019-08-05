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

//! An environment which is passed downward into the widget tree.

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use crate::kurbo::{Point, Rect, Size};
use crate::piet::Color;

use crate::Data;

/// An environment passed down through all widget traversals.
///
/// All widget methods have access to an environment, and it is passed
/// downwards during traversals.
///
/// A widget can retrieve theme parameters (colors, dimensions, etc.). In
/// addition, it can pass custom data down to all descendants. An important
/// example of the latter is setting a value for enabled/disabled status
/// so that an entire subtree can be disabled ("grayed out") with one
/// setting.
#[derive(Clone, Default)]
pub struct Env(Arc<EnvImpl>);

#[derive(Clone, Default)]
struct EnvImpl {
    map: HashMap<String, Value>,
}

/// A typed key.
///
/// This lets you retrieve values of a given type.
pub struct Key<T> {
    key: &'static str,
    value_type: PhantomData<T>,
}

// we could do some serious deriving here: the set of types that can be stored
// could be defined per-app
// Also consider Box<Any> (though this would also impact debug).
#[derive(Clone)]
pub enum Value {
    Point(Point),
    Size(Size),
    Rect(Rect),
    Color(Color),
    Float(f64),
    String(String),
}

impl Env {
    // TODO: want to change this to return `&V`.
    pub fn get<V: TryFrom<Value, Error = String>>(&self, key: Key<V>) -> V {
        if let Some(value) = self.0.map.get(key.key) {
            value.to_owned().into_inner_unchecked()
        } else {
            panic!("key for {} not found", key.key)
        }
    }

    // Also &V.
    pub fn try_get<V: TryFrom<Value, Error = String>>(&self, key: Key<V>) -> Option<V> {
        self.0
            .map
            .get(key.key)
            .map(|value| value.to_owned().into_inner_unchecked())
    }

    /// Adds a key/value, acting like a builder.
    pub fn adding<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Env {
        let env = Arc::make_mut(&mut self.0);
        env.map.insert(key.into(), value.into());
        self
    }

    pub fn set<K: Into<String>, V: Into<Value>>(&mut self, key: K, value: V) {
        let env = Arc::make_mut(&mut self.0);
        let value = value.into();
        let key = key.into();
        // TODO: use of Entry might be more efficient
        if let Some(existing) = env.map.get(&key) {
            if !existing.is_same_type(&value) {
                panic!(
                    "Invalid type for key '{}': {:?} differs in kind from {:?}",
                    key, existing, value
                );
            }
        }
        env.map.insert(key, value);
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Value::Point(p) => write!(f, "Point {:?}", p),
            Value::Size(s) => write!(f, "Size {:?}", s),
            Value::Rect(r) => write!(f, "Rect {:?}", r),
            Value::Color(c) => write!(f, "Color {:?}", c),
            Value::Float(x) => write!(f, "Float {}", x),
            Value::String(s) => write!(f, "String {:?}", s),
        }
    }
}

impl<T> Key<T> {
    pub const fn new(key: &'static str) -> Self {
        Key {
            key,
            value_type: PhantomData,
        }
    }
}

impl Value {
    /// Panics if `self` is not an instance of `V`.
    pub fn into_inner_unchecked<V: TryFrom<Value, Error = String>>(self) -> V {
        match self.try_into() {
            Ok(v) => v,
            Err(s) => panic!("{}", s),
        }
    }

    fn is_same_type(&self, other: &Value) -> bool {
        use Value::*;
        match (self, other) {
            (Point(_), Point(_)) => true,
            (Size(_), Size(_)) => true,
            (Rect(_), Rect(_)) => true,
            (Color(_), Color(_)) => true,
            (Float(_), Float(_)) => true,
            (String(_), String(_)) => true,
            _ => false,
        }
    }
}

impl Data for Value {
    fn same(&self, other: &Value) -> bool {
        use Value::*;
        match (self, other) {
            (Point(p1), Point(p2)) => p1.x.same(&p2.x) && p1.y.same(&p2.y),
            (Rect(r1), Rect(r2)) => {
                r1.x0.same(&r2.x0) && r1.y0.same(&r2.y0) && r1.x1.same(&r2.x1) && r1.y1.same(&r2.y1)
            }
            (Size(s1), Size(s2)) => s1.width.same(&s2.width) && s1.height.same(&s2.height),
            (Color(c1), Color(c2)) => c1.as_rgba_u32() == c2.as_rgba_u32(),
            (Float(f1), Float(f2)) => f1.same(&f2),
            (String(s1), String(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl Data for Env {
    fn same(&self, other: &Env) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.deref().same(other.0.deref())
    }
}

impl Data for EnvImpl {
    fn same(&self, other: &EnvImpl) -> bool {
        if self.map.len() != other.map.len() {
            return false;
        }
        for (k, v1) in self.map.iter() {
            if let Some(v2) = other.map.get(k) {
                if !v1.same(v2) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl<T> From<Key<T>> for String {
    fn from(src: Key<T>) -> String {
        String::from(src.key)
    }
}

// This came from a land where these things were copy. I think
// we want do deal with both references and owned values.
macro_rules! impl_try_from {
    ($ty:ty, $var:ident) => {
        impl TryFrom<Value> for $ty {
            type Error = String;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                match value {
                    Value::$var(f) => Ok(f),
                    other => Err(format!(
                        "incorrect Value type. Expected {}, found {:?}",
                        stringify!($var),
                        other
                    )),
                }
            }
        }

        impl Into<Value> for $ty {
            fn into(self) -> Value {
                Value::$var(self)
            }
        }
    };
}

impl_try_from!(f64, Float);
impl_try_from!(Color, Color);
impl_try_from!(Rect, Rect);
impl_try_from!(Point, Point);
impl_try_from!(Size, Size);
impl_try_from!(String, String);
