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

// TODO: remove this
#![allow(unused)]

use std::collections::HashMap;
use std::ops::{Deref, Index};

use crate::piet::Color;

use crate::Data;

// Discussion question: since druid is pretty rigorously single-threaded,
// maybe we should use Rc instead of Arc.
use std::sync::Arc;

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
#[derive(Clone)]
pub struct Env(Arc<EnvImpl>);

struct EnvImpl {
    theme: EnvValue,
    // This is always a map, maybe should be a map here.
    values: EnvValue,
}

/// A value that can be stored in an environment.
/// 
/// This is an enum with a variety of common types.
#[derive(Clone)]
pub enum EnvValue {
    Float(f64),
    Bool(bool),
    Color(Color),
    String(String),
    List(Arc<Vec<EnvValue>>),
    Map(Arc<HashMap<String, EnvValue>>),
    Null,
}

impl Env {
    /// Create an environment with a theme but an empty values map.
    pub fn from_theme(theme: EnvValue) -> Env {
        Env(Arc::new(EnvImpl {
            theme,
            values: EnvValue::empty_map(),
        }))
    }

    /// Get a theme value.
    ///
    /// The value will be EnvValue::Null if the key is not present.
    // TODO: make key more flexible
    pub fn theme_value(&self, key: &str) -> &EnvValue {
        &self.0.theme[key]
    }

    /// Get a theme color.
    ///
    /// Prints a debug warning and returns a garish color if the key
    /// isn't present or is of the wrong type.
    pub fn theme_color(&self, key: &str) -> Color {
        if let Some(c) = self.theme_value(key).as_color() {
            c
        } else {
            println!("missing theme color for {}", key);
            Color::rgb24(0xff_00_ff)
        }
    }

    /// Get a theme float value.
    ///
    /// Prints a debug warning and returns 0.0 if the key isn't present
    /// or is of the wrong type.
    ///
    /// Discussion question: would it be better to panic? Should we have
    /// a more systematic error logging infrastructure? This seems good
    /// enough for now.
    pub fn theme_float(&self, key: &str) -> f64 {
        if let Some(f) = self.theme_value(key).as_float() {
            f
        } else {
            println!("missing theme float for {}", key);
            0.0
        }
    }

    /// Update a value within the environment map.
    ///
    /// Environments are considered immutable, so this returns a new
    /// environment with the value updated.
    pub fn update(&self, key: &str, value: impl Into<EnvValue>) -> Env {
        // Note: this always does the clone, we might want a variant that
        // takes `self` so doesn't.
        let values = self.0.values.to_owned().update(key, value.into());
        Env(Arc::new(EnvImpl {
            theme: self.0.theme.clone(),
            values,
        }))
    }
}

impl Data for Env {
    fn same(&self, other: &Env) -> bool {
        self.0.theme.same(&other.0.theme) && self.0.values.same(&other.0.values)
    }
}

impl EnvValue {
    /// An empty map as an environment value.
    pub fn empty_map() -> EnvValue {
        EnvValue::Map(Arc::new(HashMap::new()))
    }

    /// Provide the value as a float, if that is its type.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            EnvValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Provide the value as a boolean, if that is its type.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            EnvValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Provide the value as a color, if that is its type.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            EnvValue::Color(c) => Some(c.to_owned()),
            _ => None,
        }
    }

    /// Provide the value as a string, if that is its type.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            EnvValue::String(s) => Some(s),
            _ => None,
        }
    }

    // Note: for the key here, we probably want to do something like the KeyPath
    // that's in value.rs.
    /// Update the value for a key in a map.
    ///
    /// As environment values are considered immutable, this returns
    /// a new value with the update applied.
    pub fn update(self, key: impl Into<String>, value: impl Into<EnvValue>) -> EnvValue {
        match self {
            EnvValue::Map(mut m) => {
                let mut m_ref = Arc::make_mut(&mut m);
                m_ref.insert(key.into(), value.into());
                EnvValue::Map(m)
            }
            _ => EnvValue::Null,
        }
    }
}

impl From<f64> for EnvValue {
    fn from(x: f64) -> EnvValue {
        EnvValue::Float(x)
    }
}

impl From<bool> for EnvValue {
    fn from(x: bool) -> EnvValue {
        EnvValue::Bool(x)
    }
}

impl From<Color> for EnvValue {
    fn from(x: Color) -> EnvValue {
        EnvValue::Color(x)
    }
}

impl<'a> From<&'a str> for EnvValue {
    fn from(x: &str) -> EnvValue {
        EnvValue::String(x.to_owned())
    }
}

impl<'a> From<String> for EnvValue {
    fn from(x: String) -> EnvValue {
        EnvValue::String(x)
    }
}

impl From<HashMap<String, EnvValue>> for EnvValue {
    fn from(x: HashMap<String, EnvValue>) -> EnvValue {
        EnvValue::Map(x.into())
    }
}

// TODO: make this more general (follow serde_json::Value). This
// is a sketch.
impl<'a> Index<&'a str> for EnvValue {
    type Output = EnvValue;
    fn index(&self, key: &str) -> &EnvValue {
        match self {
            EnvValue::Map(m) => {
                if let Some(v) = m.get(key) {
                    v
                } else {
                    &EnvValue::Null
                }
            }
            _ => &EnvValue::Null,
        }
    }
}

impl Data for EnvValue {
    fn same(&self, other: &EnvValue) -> bool {
        match (self, other) {
            (EnvValue::Float(f1), EnvValue::Float(f2)) => f1.same(f2),
            (EnvValue::Color(c1), EnvValue::Color(c2)) => {
                // Note: when `Color` gets richer, this also needs to change.
                // Consider implementing equality (or sameness, considering the
                // possibility of float NaN values) on `piet::Color`.
                c1.as_rgba32() == c2.as_rgba32()
            }
            (EnvValue::String(s1), EnvValue::String(s2)) => s1 == s2,
            (EnvValue::List(l1), EnvValue::List(l2)) => l1.same(l2),
            (EnvValue::Map(m1), EnvValue::Map(m2)) => {
                // We traverse here because of the high likelihood that
                // successive traversals to child widgets will have the
                // same value as before, but update doesn't do hash consing.
                // (If we did hash consing, we could just use pointer eq)
                if Arc::ptr_eq(m1, m2) {
                    return true;
                }
                if m1.len() != m2.len() {
                    return false;
                }
                for (k, v1) in m1.iter() {
                    if let Some(v2) = m2.get(k) {
                        if !v1.same(v2) {
                            return false;
                        }
                    }
                }
                true
            }
            (EnvValue::Null, EnvValue::Null) => true,
            _ => false,
        }
    }
}
