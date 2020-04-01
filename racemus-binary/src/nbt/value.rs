use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Arc<[u8]>),
    String(Arc<str>),
    List(Arc<[Value]>),
    Compound(HashMap<Arc<str>, Value>),
    IntArray(Arc<[i32]>),
    LongArray(Arc<[i64]>),
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::Byte(value)
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Value::Short(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Long(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Double(value)
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Value::ByteArray(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.into())
    }
}

impl From<&[Value]> for Value {
    fn from(value: &[Value]) -> Self {
        Value::List(value.into())
    }
}

impl From<&[i32]> for Value {
    fn from(value: &[i32]) -> Self {
        Value::IntArray(value.into())
    }
}

impl From<&[i64]> for Value {
    fn from(value: &[i64]) -> Self {
        Value::LongArray(value.into())
    }
}
