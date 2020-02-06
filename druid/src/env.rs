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

use std::any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{Color, LinearGradient};

use crate::localization::L10nManager;
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
#[derive(Clone)]
pub struct Env(Arc<EnvImpl>);

#[derive(Clone)]
struct EnvImpl {
    map: HashMap<String, Value>,
    debug_colors: Vec<Color>,
    l10n: Arc<L10nManager>,
}

/// A typed key.
///
/// This lets you retrieve values of a given type. The parameter
/// implements [`ValueType`]. For "expensive" types, this is a reference,
/// so the type for a string is `Key<&str>`.
///
/// [`ValueType`]: trait.ValueType.html
pub struct Key<T> {
    key: &'static str,
    value_type: PhantomData<T>,
}

// we could do some serious deriving here: the set of types that can be stored
// could be defined per-app
// Also consider Box<Any> (though this would also impact debug).
/// A dynamic type representing all values that can be stored in an environment.
#[derive(Clone)]
pub enum Value {
    Point(Point),
    Size(Size),
    Rect(Rect),
    Color(Color),
    LinearGradient(Arc<LinearGradient>),
    Float(f64),
    Bool(bool),
    UnsignedInt(u64),
    String(String),
}

/// Values which can be stored in an environment.
///
/// Note that for "expensive" types this is the reference. For example,
/// for strings, this trait is implemented on `&'a str`. The trait is
/// parametrized on a lifetime so that it can be used for references in
/// this way.
pub trait ValueType<'a>: Sized {
    /// The corresponding owned type.
    type Owned: Into<Value>;

    /// Attempt to convert the generic `Value` into this type.
    fn try_from_value(v: &'a Value) -> Result<Self, ValueTypeError>;
}

/// The error type for environment access.
///
/// This error is expected to happen rarely, if ever, as it only
/// happens when the string part of keys collide but the types
/// mismatch.
#[derive(Debug, Clone)]
pub struct ValueTypeError {
    expected: &'static str,
    found: Value,
}

impl Env {
    /// Gets a value from the environment, expecting it to be present.
    ///
    /// Note that the return value is a reference for "expensive" types such
    /// as strings, but an ordinary value for "cheap" types such as numbers
    /// and colors.
    ///
    /// # Panics
    ///
    /// Panics if the key is not found, or if it is present with the wrong type.
    pub fn get<'a, V: ValueType<'a>>(&'a self, key: Key<V>) -> V {
        if let Some(value) = self.0.map.get(key.key) {
            value.to_inner_unchecked()
        } else {
            panic!("key for {} not found", key.key)
        }
    }

    /// Gets a value from the environment.
    ///
    /// # Panics
    ///
    /// Panics if the value for the key is found, but has the wrong type.
    pub fn try_get<'a, V: ValueType<'a>>(&'a self, key: Key<V>) -> Option<V> {
        self.0
            .map
            .get(key.key)
            .map(|value| value.to_inner_unchecked())
    }

    /// Adds a key/value, acting like a builder.
    pub fn adding<'a, V: ValueType<'a>>(mut self, key: Key<V>, value: impl Into<V::Owned>) -> Env {
        let env = Arc::make_mut(&mut self.0);
        env.map.insert(key.into(), value.into().into());
        self
    }

    /// Sets a value in an environment.
    ///
    /// # Panics
    ///
    /// Panics if the environment already has a value for the key, but it is
    /// of a different type.
    pub fn set<'a, V: ValueType<'a>>(&'a mut self, key: Key<V>, value: impl Into<V::Owned>) {
        let env = Arc::make_mut(&mut self.0);
        let value = value.into().into();
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

    /// Returns a reference to the [`L10nManager`], which handles localization
    /// resources.
    ///
    /// [`L10nManager`]: struct.L10nManager.html
    pub(crate) fn localization_manager(&self) -> &L10nManager {
        &self.0.l10n
    }

    /// Given an id, returns one of 18 distinct colors
    #[doc(hidden)]
    pub fn get_debug_color(&self, id: u64) -> Color {
        let color_num = id as usize % self.0.debug_colors.len();
        self.0.debug_colors[color_num].clone()
    }

    /// State for whether or not to paint colorful rectangles for layout
    /// debugging.
    ///
    /// Set by the `debug_paint_layout()` method on [`AppLauncher`]'.
    ///
    /// [`AppLauncher`]: struct.AppLauncher.html
    pub(crate) const DEBUG_PAINT: Key<bool> = Key::new("debug_paint");
}

impl<T> Key<T> {
    /// Create a new strongly typed `Key` with the given string value.
    /// The type of the key will be inferred.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::Key;
    /// use druid::piet::Color;
    ///
    /// let float_key: Key<f64> = Key::new("a.very.good.float");
    /// let color_key: Key<Color> = Key::new("a.very.nice.color");
    /// ```
    pub const fn new(key: &'static str) -> Self {
        Key {
            key,
            value_type: PhantomData,
        }
    }
}

impl Value {
    /// Get a reference to the inner object.
    ///
    /// # Panics
    ///
    /// Panics when the value variant doesn't match the provided type.
    pub fn to_inner_unchecked<'a, V: ValueType<'a>>(&'a self) -> V {
        match ValueType::try_from_value(self) {
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
            (LinearGradient(_), LinearGradient(_)) => true,
            (Float(_), Float(_)) => true,
            (Bool(_), Bool(_)) => true,
            (UnsignedInt(_), UnsignedInt(_)) => true,
            (String(_), String(_)) => true,
            _ => false,
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Value::Point(p) => write!(f, "Point {:?}", p),
            Value::Size(s) => write!(f, "Size {:?}", s),
            Value::Rect(r) => write!(f, "Rect {:?}", r),
            Value::Color(c) => write!(f, "Color {:?}", c),
            Value::LinearGradient(g) => write!(f, "LinearGradient {:?}", g),
            Value::Float(x) => write!(f, "Float {}", x),
            Value::Bool(b) => write!(f, "Bool {}", b),
            Value::UnsignedInt(x) => write!(f, "UnsignedInt {}", x),
            Value::String(s) => write!(f, "String {:?}", s),
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
            (LinearGradient(g1), LinearGradient(g2)) => Arc::ptr_eq(g1, g2),
            (Float(f1), Float(f2)) => f1.same(&f2),
            (Bool(b1), Bool(b2)) => b1 == b2,
            (UnsignedInt(f1), UnsignedInt(f2)) => f1.same(&f2),
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
        self.map.len() == other.map.len()
            && self
                .map
                .iter()
                .all(|(k, v1)| other.map.get(k).map(|v2| v1.same(v2)).unwrap_or(false))
    }
}

impl Default for Env {
    fn default() -> Self {
        let l10n = L10nManager::new(vec!["builtin.ftl".into()], "./resources/i18n/");

        // Colors are from https://sashat.me/2017/01/11/list-of-20-simple-distinct-colors/
        // They're picked for visual distinction and accessbility (99 percent)
        let debug_colors = vec![
            Color::rgb8(230, 25, 75),
            Color::rgb8(60, 180, 75),
            Color::rgb8(255, 225, 25),
            Color::rgb8(0, 130, 200),
            Color::rgb8(245, 130, 48),
            Color::rgb8(70, 240, 240),
            Color::rgb8(240, 50, 230),
            Color::rgb8(250, 190, 190),
            Color::rgb8(0, 128, 128),
            Color::rgb8(230, 190, 255),
            Color::rgb8(170, 110, 40),
            Color::rgb8(255, 250, 200),
            Color::rgb8(128, 0, 0),
            Color::rgb8(170, 255, 195),
            Color::rgb8(0, 0, 128),
            Color::rgb8(128, 128, 128),
            Color::rgb8(255, 255, 255),
            Color::rgb8(0, 0, 0),
        ];

        let inner = EnvImpl {
            l10n: Arc::new(l10n),
            map: HashMap::new(),
            debug_colors,
        };

        Env(Arc::new(inner)).adding(Env::DEBUG_PAINT, false)
    }
}

impl<T> From<Key<T>> for String {
    fn from(src: Key<T>) -> String {
        String::from(src.key)
    }
}

impl ValueTypeError {
    fn new(expected: &'static str, found: Value) -> ValueTypeError {
        ValueTypeError { expected, found }
    }
}
impl std::fmt::Display for ValueTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Incorrect value type: expected {} found {:?}",
            self.expected, self.found
        )
    }
}

impl std::error::Error for ValueTypeError {}

/// Use this macro for types which are cheap to clone (ie all `Copy` types).
macro_rules! impl_value_type_owned {
    ($ty:ty, $var:ident) => {
        impl<'a> ValueType<'a> for $ty {
            type Owned = $ty;
            fn try_from_value(value: &Value) -> Result<Self, ValueTypeError> {
                match value {
                    Value::$var(f) => Ok(f.to_owned()),
                    other => Err(ValueTypeError::new(any::type_name::<$ty>(), other.clone())),
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

/// Use this macro for types which require allocation but are not too
/// expensive to clone.
macro_rules! impl_value_type_borrowed {
    ($ty:ty, $owned:ty, $var:ident) => {
        impl<'a> ValueType<'a> for &'a $ty {
            type Owned = $owned;
            fn try_from_value(value: &'a Value) -> Result<Self, ValueTypeError> {
                match value {
                    Value::$var(f) => Ok(f),
                    other => Err(ValueTypeError::new(any::type_name::<$ty>(), other.clone())),
                }
            }
        }

        impl Into<Value> for $owned {
            fn into(self) -> Value {
                Value::$var(self)
            }
        }
    };
}

/// Use this macro for types that would be expensive to clone; they
/// are stored as an `Arc<>`.
macro_rules! impl_value_type_arc {
    ($ty:ty, $var:ident) => {
        impl<'a> ValueType<'a> for &'a $ty {
            type Owned = $ty;
            fn try_from_value(value: &'a Value) -> Result<Self, ValueTypeError> {
                match value {
                    Value::$var(f) => Ok(f),
                    other => Err(ValueTypeError::new(any::type_name::<$ty>(), other.clone())),
                }
            }
        }

        impl Into<Value> for $ty {
            fn into(self) -> Value {
                Value::$var(Arc::new(self))
            }
        }

        impl Into<Value> for Arc<$ty> {
            fn into(self) -> Value {
                Value::$var(self)
            }
        }
    };
}

impl_value_type_owned!(f64, Float);
impl_value_type_owned!(bool, Bool);
impl_value_type_owned!(u64, UnsignedInt);
impl_value_type_owned!(Color, Color);
impl_value_type_owned!(Rect, Rect);
impl_value_type_owned!(Point, Point);
impl_value_type_owned!(Size, Size);
impl_value_type_borrowed!(str, String, String);
impl_value_type_arc!(LinearGradient, LinearGradient);
