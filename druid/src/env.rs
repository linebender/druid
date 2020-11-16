// Copyright 2019 The Druid Authors.
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
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use crate::localization::L10nManager;
use crate::text::FontDescriptor;
use crate::{ArcStr, Color, Data, Insets, Point, Rect, Size};

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
///
/// [`EnvScope`] can be used to override parts of `Env` for its descendants.
///
/// # Important
/// It is the programmer's responsibility to ensure that the environment
/// is used correctly. See [`Key`] for an example.
/// - [`Key`]s should be `const`s with unique names
/// - [`Key`]s must always be set before they are used.
/// - Values can only be overwritten by values of the same type.
///
/// [`EnvScope`]: widget/struct.EnvScope.html
/// [`Key`]: struct.Key.html
#[derive(Clone)]
pub struct Env(Arc<EnvImpl>);

#[derive(Clone)]
struct EnvImpl {
    map: HashMap<ArcStr, Value>,
    debug_colors: Vec<Color>,
    l10n: Arc<L10nManager>,
}

/// A typed [`Env`] key.
///
/// This lets you retrieve values of a given type. The parameter
/// implements [`ValueType`]. For "expensive" types, this is a reference,
/// so the type for a string is `Key<&str>`.
///
/// # Examples
///
/// ```
///# use druid::{Key, Color, WindowDesc, AppLauncher, widget::Label};
/// const IMPORTANT_LABEL_COLOR: Key<Color> = Key::new("org.linebender.example.important-label-color");
///
/// fn important_label() -> Label<()> {
///     Label::new("Warning!").with_text_color(IMPORTANT_LABEL_COLOR)
/// }
///
/// fn main() {
///     let main_window = WindowDesc::new(important_label);
///
///     AppLauncher::with_window(main_window)
///         .configure_env(|env, _state| {
///             // The `Key` must be set before it is used.
///             env.set(IMPORTANT_LABEL_COLOR, Color::rgb(1.0, 0.0, 0.0));
///         });
/// }
/// ```
///
/// [`ValueType`]: trait.ValueType.html
/// [`Env`]: struct.Env.html
#[derive(Clone, Debug, PartialEq)]
pub struct Key<T> {
    key: &'static str,
    value_type: PhantomData<T>,
}

// we could do some serious deriving here: the set of types that can be stored
// could be defined per-app
// Also consider Box<Any> (though this would also impact debug).
/// A dynamic type representing all values that can be stored in an environment.
#[derive(Clone, Data, PartialEq)]
#[allow(missing_docs)]
// ANCHOR: value_type
pub enum Value {
    Point(Point),
    Size(Size),
    Rect(Rect),
    Insets(Insets),
    Color(Color),
    Float(f64),
    Bool(bool),
    UnsignedInt(u64),
    String(ArcStr),
    Font(FontDescriptor),
}
// ANCHOR_END: value_type

/// Either a concrete `T` or a [`Key<T>`] that can be resolved in the [`Env`].
///
/// This is a way to allow widgets to interchangeably use either a specific
/// value or a value from the environment for some purpose.
///
/// [`Key<T>`]: struct.Key.html
/// [`Env`]: struct.Env.html
#[derive(Clone, PartialEq, Debug)]
pub enum KeyOrValue<T> {
    /// A concrete [`Value`] of type `T`.
    ///
    /// [`Value`]: enum.Value.html
    Concrete(T),
    /// A [`Key<T>`] that can be resolved to a value in the [`Env`].
    ///
    /// [`Key<T>`]: struct.Key.html
    /// [`Env`]: struct.Env.html
    Key(Key<T>),
}

/// A trait for anything that can resolve a value of some type from the [`Env`].
///
/// This is a generalization of the idea of [`KeyOrValue`], mostly motivated
/// by wanting to improve the API used for checking if items in the [`Env`] have changed.
///
/// [`Env`]: struct.Env.html
/// [`KeyOrValue`]: enum.KeyOrValue.html
pub trait KeyLike<T> {
    /// Returns `true` if this item has changed between the old and new [`Env`].
    ///
    /// [`Env`]: struct.Env.html
    fn changed(&self, old: &Env, new: &Env) -> bool;
}

impl<T: ValueType> KeyLike<T> for Key<T> {
    fn changed(&self, old: &Env, new: &Env) -> bool {
        !old.get_untyped(self).same(new.get_untyped(self))
    }
}

impl<T> KeyLike<T> for KeyOrValue<T> {
    fn changed(&self, old: &Env, new: &Env) -> bool {
        match self {
            KeyOrValue::Concrete(_) => false,
            KeyOrValue::Key(key) => !old.get_untyped(key).same(new.get_untyped(key)),
        }
    }
}

/// Values which can be stored in an environment.
pub trait ValueType: Sized + Clone + Into<Value> {
    /// Attempt to convert the generic `Value` into this type.
    fn try_from_value(v: &Value) -> Result<Self, ValueTypeError>;
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

/// An error type for when a key is missing from the [`Env`].
///
/// [`Env`]: struct.Env.html
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MissingKeyError {
    /// The raw key.
    key: Arc<str>,
}

impl Env {
    /// State for whether or not to paint colorful rectangles for layout
    /// debugging.
    ///
    /// Set by the `debug_paint_layout()` method on [`WidgetExt`]'.
    ///
    /// [`WidgetExt`]: trait.WidgetExt.html
    pub(crate) const DEBUG_PAINT: Key<bool> = Key::new("org.linebender.druid.built-in.debug-paint");

    /// State for whether or not to paint `WidgetId`s, for event debugging.
    ///
    /// Set by the `debug_widget_id()` method on [`WidgetExt`].
    ///
    /// [`WidgetExt`]: trait.WidgetExt.html
    pub(crate) const DEBUG_WIDGET_ID: Key<bool> =
        Key::new("org.linebender.druid.built-in.debug-widget-id");

    /// A key used to tell widgets to print additional debug information.
    ///
    /// This does nothing by default; however you can check this key while
    /// debugging a widget to limit println spam.
    ///
    /// For convenience, this key can be set with the [`WidgetExt::debug_widget`]
    /// method.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use druid::Env;
    /// # let env = Env::default();
    /// # let widget_id = 0;
    /// # let my_rect = druid::Rect::ZERO;
    /// if env.get(Env::DEBUG_WIDGET) {
    ///     eprintln!("widget {:?} bounds: {:?}", widget_id, my_rect);
    /// }
    /// ```
    ///
    /// [`WidgetExt::debug_widget`]: trait.WidgetExt.html#method.debug_widget
    pub const DEBUG_WIDGET: Key<bool> = Key::new("org.linebender.druid.built-in.debug-widget");

    /// Gets a value from the environment, expecting it to be present.
    ///
    /// Note that the return value is a reference for "expensive" types such
    /// as strings, but an ordinary value for "cheap" types such as numbers
    /// and colors.
    ///
    /// # Panics
    ///
    /// Panics if the key is not found, or if it is present with the wrong type.
    pub fn get<V: ValueType>(&self, key: impl Borrow<Key<V>>) -> V {
        match self.try_get(key) {
            Ok(value) => value,
            Err(err) => panic!("{}", err),
        }
    }

    /// Trys to get a value from the environment.
    ///
    /// If the value is not found, the raw key is returned as the error.
    ///
    /// # Panics
    ///
    /// Panics if the value for the key is found, but has the wrong type.
    pub fn try_get<V: ValueType>(&self, key: impl Borrow<Key<V>>) -> Result<V, MissingKeyError> {
        self.0
            .map
            .get(key.borrow().key)
            .map(|value| value.to_inner_unchecked())
            .ok_or(MissingKeyError {
                key: key.borrow().key.into(),
            })
    }

    /// Gets a value from the environment, in its encapsulated [`Value`] form,
    /// expecting the key to be present.
    ///
    /// *WARNING:* This is not intended for general use, but only for inspecting an `Env` e.g.
    /// for debugging, theme editing, and theme loading.
    ///
    /// # Panics
    ///
    /// Panics if the key is not found.
    ///
    /// [`Value`]: enum.Value.html
    pub fn get_untyped<V>(&self, key: impl Borrow<Key<V>>) -> &Value {
        match self.try_get_untyped(key) {
            Ok(val) => val,
            Err(err) => panic!("{}", err),
        }
    }

    /// Gets a value from the environment, in its encapsulated [`Value`] form,
    /// returning `None` if a value isn't found.
    ///
    /// # Note
    /// This is not intended for general use, but only for inspecting an `Env`
    /// e.g. for debugging, theme editing, and theme loading.
    ///
    /// [`Value`]: enum.Value.html
    pub fn try_get_untyped<V>(&self, key: impl Borrow<Key<V>>) -> Result<&Value, MissingKeyError> {
        self.0.map.get(key.borrow().key).ok_or(MissingKeyError {
            key: key.borrow().key.into(),
        })
    }

    /// Gets the entire contents of the `Env`, in key-value pairs.
    ///
    /// *WARNING:* This is not intended for general use, but only for inspecting an `Env` e.g.
    /// for debugging, theme editing, and theme loading.
    pub fn get_all(&self) -> impl ExactSizeIterator<Item = (&ArcStr, &Value)> {
        self.0.map.iter()
    }

    /// Adds a key/value, acting like a builder.
    pub fn adding<V: ValueType>(mut self, key: Key<V>, value: impl Into<V>) -> Env {
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
    pub fn set<V: ValueType>(&mut self, key: Key<V>, value: impl Into<V>) {
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
    /// let float_key: Key<f64> = Key::new("org.linebender.example.a.very.good.float");
    /// let color_key: Key<Color> = Key::new("org.linebender.example.a.very.nice.color");
    /// ```
    pub const fn new(key: &'static str) -> Self {
        Key {
            key,
            value_type: PhantomData,
        }
    }
}

impl Key<()> {
    /// Create an untyped `Key` with the given string value.
    ///
    /// *WARNING:* This is not for general usage - it's only useful
    /// for inspecting the contents of an [`Env`]  - this is expected to be
    /// used for debugging, loading, and manipulating themes.
    ///
    /// [`Env`]: struct.Env.html
    pub const fn untyped(key: &'static str) -> Self {
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
    pub fn to_inner_unchecked<V: ValueType>(&self) -> V {
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
            (Insets(_), Insets(_)) => true,
            (Color(_), Color(_)) => true,
            (Float(_), Float(_)) => true,
            (Bool(_), Bool(_)) => true,
            (UnsignedInt(_), UnsignedInt(_)) => true,
            (String(_), String(_)) => true,
            (Font(_), Font(_)) => true,
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
            Value::Insets(i) => write!(f, "Insets {:?}", i),
            Value::Color(c) => write!(f, "Color {:?}", c),
            Value::Float(x) => write!(f, "Float {}", x),
            Value::Bool(b) => write!(f, "Bool {}", b),
            Value::UnsignedInt(x) => write!(f, "UnsignedInt {}", x),
            Value::String(s) => write!(f, "String {:?}", s),
            Value::Font(font) => write!(f, "Font {:?}", font),
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

        let env = Env(Arc::new(inner))
            .adding(Env::DEBUG_PAINT, false)
            .adding(Env::DEBUG_WIDGET_ID, false)
            .adding(Env::DEBUG_WIDGET, false);

        crate::theme::add_to_env(env)
    }
}

impl<T> From<Key<T>> for ArcStr {
    fn from(src: Key<T>) -> ArcStr {
        ArcStr::from(src.key)
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

impl MissingKeyError {
    /// The raw key that was missing.
    pub fn raw_key(&self) -> &str {
        &self.key
    }
}

impl std::fmt::Display for MissingKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Missing key: '{}'", self.key)
    }
}

impl std::error::Error for ValueTypeError {}
impl std::error::Error for MissingKeyError {}

/// Use this macro for types which are cheap to clone (ie all `Copy` types).
macro_rules! impl_value_type {
    ($ty:ty, $var:ident) => {
        impl ValueType for $ty {
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

impl_value_type!(f64, Float);
impl_value_type!(bool, Bool);
impl_value_type!(u64, UnsignedInt);
impl_value_type!(Color, Color);
impl_value_type!(Rect, Rect);
impl_value_type!(Point, Point);
impl_value_type!(Size, Size);
impl_value_type!(Insets, Insets);
impl_value_type!(ArcStr, String);
impl_value_type!(FontDescriptor, Font);

impl<T: ValueType> KeyOrValue<T> {
    /// Resolve the concrete type `T` from this `KeyOrValue`, using the provided
    /// [`Env`] if required.
    ///
    /// [`Env`]: struct.Env.html
    pub fn resolve(&self, env: &Env) -> T {
        match self {
            KeyOrValue::Concrete(ref value) => value.to_owned(),
            KeyOrValue::Key(key) => env.get(key),
        }
    }
}

impl<T: Into<Value>> From<T> for KeyOrValue<T> {
    fn from(value: T) -> KeyOrValue<T> {
        KeyOrValue::Concrete(value)
    }
}

impl<T: ValueType> From<Key<T>> for KeyOrValue<T> {
    fn from(key: Key<T>) -> KeyOrValue<T> {
        KeyOrValue::Key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_key_or_value() {
        const MY_KEY: Key<ArcStr> = Key::new("org.linebender.test.my-string-key");
        let env = Env::default().adding(MY_KEY, "Owned");
        assert_eq!(env.get(MY_KEY).as_ref(), "Owned");

        let key: KeyOrValue<ArcStr> = MY_KEY.into();
        let value: KeyOrValue<ArcStr> = ArcStr::from("Owned").into();

        assert_eq!(key.resolve(&env), value.resolve(&env));
    }

    #[test]
    fn key_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Key<()>>();
    }
}
