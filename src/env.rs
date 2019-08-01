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

pub struct Env(Arc<EnvImpl>);

pub struct EnvImpl {
    theme: EnvValue,
}

#[derive(Clone)]
pub enum EnvValue {
    Float(f64),
    String(String),
    List(Arc<Vec<EnvValue>>),
    Map(Arc<HashMap<String, EnvValue>>),
    Null,
}

impl EnvValue {
    pub fn as_float(&self) -> Option<f64> {
        match self {
            EnvValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            EnvValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn update(self, key: impl Into<String>, value: EnvValue) -> EnvValue {
        match self {
            EnvValue::Map(mut m) => {
                let mut m_ref = Arc::make_mut(&mut m);
                m_ref.insert(key.into(), value);
                EnvValue::Map(m)
            }
            _ => EnvValue::Null,
        }
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
