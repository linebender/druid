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

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;

pub use defaults::colors;

//TODO: any key that is accepted has to have a provided default value

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    origin: Point,
    size: Size,
}
type Color = u32;

pub struct Environment {
    pub theme: Variables,
}

impl std::default::Default for Environment {
    fn default() -> Self {
        Environment {
            theme: defaults::default_theme(),
        }
    }
}

pub struct Key<T> {
    key: &'static str,
    value_type: PhantomData<T>,
}

impl<T> Key<T> {
    pub const fn new(key: &'static str) -> Self {
        Key {
            key,
            value_type: PhantomData,
        }
    }
}

// we could do some serious deriving here: the set of types that can be stored
// could be defined per-app
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Point(Point),
    Size(Size),
    Rect(Rect),
    Color(Color),
    Float(f64),
}

impl Value {
    /// Panics if `self` is not an instance of `V`.
    pub fn into_inner_unchecked<V: TryFrom<Value, Error = String>>(self) -> V {
        match self.try_into() {
            Ok(v) => v,
            Err(s) => panic!("{}", s),
        }
    }
}

/// A set of typed key/value pairs
#[derive(Debug, Default, Clone)]
pub struct Variables {
    store: HashMap<String, Value>,
}

impl Variables {
    pub fn new() -> Self {
        Variables {
            store: HashMap::new(),
        }
    }

    /// Adds a key/value, acting like a builder.
    pub fn adding<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        self.store.insert(key.into(), value.into());
        self
    }

    pub fn get<V: TryFrom<Value, Error = String>>(&self, key: Key<V>) -> V {
        let value = match self.store.get(*&key.key) {
            Some(v) => v,
            None => panic!("No Variables key '{}'", key.key),
        };
        value.into_inner_unchecked()
    }
}

impl<T> From<Key<T>> for String {
    fn from(src: Key<T>) -> String {
        String::from(src.key)
    }
}

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

pub mod defaults {

    pub mod colors {
        use super::{Color, Key};
        pub const BACKGROUND: Key<Color> = Key::new("io.xi-editor.background_color");
        pub const TEXT: Key<Color> = Key::new("io.xi-editor.text_color");
        pub const TINT: Key<Color> = Key::new("io.xi-editor.tint_color");
        pub const DIM_TEXT: Key<Color> = Key::new("io.xi-editor.text_color_dim");
        pub const HIGHTLIGHT: Key<Color> = Key::new("io.xi-editor.highlight_color");
        pub const SELECTED_ITEM: Key<Color> = Key::new("io.xi-editor.selected_item_color");
        pub const BUTTON_DOWN: Key<Color> = Key::new("io.xi-editor.button_down_color");
    }

    use super::{Color, Environment, Key, Variables};
    use colors::*;

    pub fn default_theme() -> Variables {
        Variables::new()
            .adding(BACKGROUND, 0x_24_24_24_FF)
            .adding(TEXT, 0x_EE_EE_EE_FF)
            .adding(HIGHTLIGHT, 0xfa_fa_fa_ff)
            .adding(TINT, 0x6a_6a_6a_ff)
            .adding(BUTTON_DOWN, 0x_04_24_84_ff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Expected Color, found Point")]
    fn smoke_test() {
        let point = Point { x: 2.0, y: 5.0 };

        let env = Variables::default().adding("my_point", point);

        let key = Key::<Point>::new("my_point");
        assert_eq!(env.get(key), point);

        let key = Key::<Color>::new("my_point");
        assert_eq!(env.get(key), 0x00);
    }
}
