use std::any::{Any, TypeId};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),

    //U8(u8),
    //U16(u16),
    //U32(u32),
    U64(u64),

    //I8(i8),
    //I16(i16),
    //I32(i32),
    I64(i64),

    //F32(f32),
    F64(f64),

    Char(char),
    String(String),
    Object(TypeId, Rc<dyn Any>),
}

impl Value {
    fn is_same_type(&self, other: &Value) -> bool {
        use Value::*;
        matches!(
            (self, other),
            (Bool(_), Bool(_))
                | (U64(_), U64(_))
                | (I64(_), I64(_))
                | (F64(_), F64(_))
                | (Char(_), Char(_))
                | (String(_), String(_))
        ) || matches!((self, other), (Object(t1, _), Object(t2, _)) if t1 == t2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataId(u64);

impl DataId {
    const INVALID: DataId = DataId(0);

    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        DataId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Default)]
pub struct AttributeStore {
    storage: HashMap<DataId, Value>,
    //generation: usize,
    pub changes: HashSet<DataId>,
}

#[derive(Clone, Debug)]
pub struct Binding<T> {
    id: DataId,
    pub(crate) value: T,
}

impl<T> Binding<T> {
    pub fn new(value: T) -> Self {
        Binding {
            id: DataId::next(),
            value,
        }
    }

    pub fn id(&self) -> DataId {
        self.id
    }
}

impl<T> std::ops::Deref for Binding<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl AttributeStore {
    pub fn insert<T: ValueType>(&mut self, binding: &Binding<T>) {
        //assert_eq!(binding.id, DataId::INVALID, "attempt to insert initialized binding");
        let value = binding.value.to_value();
        let id = binding.id;
        //let id = DataId::next();
        assert!(
            self.storage.insert(id, value).is_none(),
            "attributes may only be inserted once"
        );
    }

    pub(crate) fn get_value<T: ValueType>(
        &self,
        binding: &Binding<T>,
    ) -> Result<T, ValueTypeError> {
        //TODO: errors: use track_caller to provide debug info?
        //or maybe better to return error to the callsite?
        self.storage
            .get(&binding.id)
            .map(T::try_from_value)
            .unwrap()
        //binding.value = value;
    }

    pub(crate) fn set<T: ValueType>(&mut self, binding: &Binding<T>, new: T) {
        match self.storage.entry(binding.id) {
            Entry::Occupied(mut e) => {
                let existing = e.get_mut();
                let new = new.to_value();
                if !existing.is_same_type(&new) {
                    panic!(
                        "value type mismatch: {}, {:?}",
                        std::any::type_name::<T>(),
                        new
                    );
                }
                *existing = new;
            }
            Entry::Vacant(_) => {
                panic!("cannot set uninitialized binding");
            }
        }
        self.changes.insert(binding.id);
    }
}

/// Types which can be stored in the attribute graph.
pub trait ValueType: Sized + Clone + std::fmt::Debug {
    /// Attempt to convert the generic `Value` into this type.
    fn try_from_value(v: &Value) -> Result<Self, ValueTypeError>;
    fn to_value(&self) -> Value;
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

macro_rules! impl_value_type {
    ($ty:ty, $var:ident) => {
        impl ValueType for $ty {
            fn try_from_value(value: &Value) -> Result<Self, ValueTypeError> {
                match value {
                    Value::$var(f) => Ok(f.to_owned()),
                    other => Err(ValueTypeError::new(
                        std::any::type_name::<$ty>(),
                        other.clone(),
                    )),
                }
            }

            fn to_value(&self) -> Value {
                Value::$var(self.clone())
            }
        }
    };
}

impl_value_type!(bool, Bool);
impl_value_type!(u64, U64);
impl_value_type!(i64, I64);
impl_value_type!(f64, F64);
impl_value_type!(String, String);
