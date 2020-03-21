use crate::{PacketReader, PacketWriter};
use async_std::io::{Read, Write};
use std::collections::HashMap;
use std::{
    io::{Error, ErrorKind},
    marker::Unpin,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Single(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    Compound(HashMap<String, Value>),
    List(Vec<Value>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl<W: Write + Unpin> PacketWriter<W> {
    #[inline]
    fn write_nbt_value(&mut self, nbt: &Value) -> Result<(), Error> {
        match nbt {
            Value::Byte(v) => {
                self.fix_i8(*v);
            }
            Value::Short(v) => {
                self.fix_i16(*v);
            }
            Value::Int(v) => {
                self.fix_i32(*v);
            }
            Value::Long(v) => {
                self.fix_i64(*v);
            }
            Value::Single(v) => {
                self.fix_f32(*v);
            }
            Value::Double(v) => {
                self.fix_f64(*v);
            }
            Value::ByteArray(v) => {
                if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                }
                self.fix_i32(v.len() as i32);
                self.fix_arr_u8(&v);
            }
            Value::String(v) => {
                if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                }
                self.fix_i32(v.len() as i32);
                self.fix_arr_char(&v);
            }
            Value::IntArray(v) => {
                if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                }
                self.fix_i32(v.len() as i32);
                for entry in v {
                    self.fix_i32(*entry);
                }
            }
            Value::LongArray(v) => {
                if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                }
                self.fix_i32(v.len() as i32);
                for entry in v {
                    self.fix_i64(*entry);
                }
            }
            Value::List(v) => {
                if v.len() == 0 {
                    self.fix_u8(0);
                    self.fix_i32(0);
                } else if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                } else {
                    self.write_nbt_tag(&v[0], None)?;
                    self.fix_i32(v.len() as i32);
                    for entry in v {
                        self.write_nbt_value(entry)?;
                    }
                }
            }
            Value::Compound(v) => {
                if v.len() > std::i32::MAX as usize {
                    return Err(ErrorKind::InvalidInput.into());
                }
                self.fix_i32(v.len() as i32);
                for (key, value) in v {
                    self.write_nbt_tag(value, Some(key))?;
                    self.write_nbt_value(value)?;
                }
                self.fix_u8(0);
            }
        }

        Ok(())
    }

    #[inline]
    fn write_nbt_tag(&mut self, nbt: &Value, name: Option<&str>) -> Result<(), Error> {
        let _ = match nbt {
            Value::Byte(_) => self.fix_u8(0x01),
            Value::Short(_) => self.fix_u8(0x02),
            Value::Int(_) => self.fix_u8(0x03),
            Value::Long(_) => self.fix_u8(0x04),
            Value::Single(_) => self.fix_u8(0x05),
            Value::Double(_) => self.fix_u8(0x06),
            Value::ByteArray(_) => self.fix_u8(0x07),
            Value::String(_) => self.fix_u8(0x08),
            Value::List(_) => self.fix_u8(0x09),
            Value::Compound(_) => self.fix_u8(0x0A),
            Value::IntArray(_) => self.fix_u8(0x0B),
            Value::LongArray(_) => self.fix_u8(0x0C),
        };

        if let Some(name) = name {
            if name.len() > std::i16::MAX as usize {
                return Err(ErrorKind::InvalidInput.into());
            }
            self.fix_i16(name.len() as i16);
            self.raw_arr_char(&name);
        }

        Ok(())
    }

    pub fn nbt(&mut self, nbt: &Value, name: &str) -> Result<&mut Self, Error> {
        self.write_nbt_tag(nbt, Some(name))?;
        self.write_nbt_value(nbt)?;
        Ok(self)
    }
}

impl From<i8> for Value {
    fn from(val: i8) -> Self {
        Self::Byte(val)
    }
}

impl From<i16> for Value {
    fn from(val: i16) -> Self {
        Self::Short(val)
    }
}

impl From<i32> for Value {
    fn from(val: i32) -> Self {
        Self::Int(val)
    }
}

impl From<i64> for Value {
    fn from(val: i64) -> Self {
        Self::Long(val)
    }
}

impl From<f32> for Value {
    fn from(val: f32) -> Self {
        Self::Single(val)
    }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self {
        Self::Double(val)
    }
}

impl From<Vec<u8>> for Value {
    fn from(val: Vec<u8>) -> Self {
        Self::ByteArray(val)
    }
}

impl From<&[u8]> for Value {
    fn from(val: &[u8]) -> Self {
        Self::ByteArray(val.to_vec())
    }
}

impl From<String> for Value {
    fn from(val: String) -> Self {
        Self::String(val)
    }
}

impl From<&str> for Value {
    fn from(val: &str) -> Self {
        Self::String(val.into())
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(val: HashMap<String, Value>) -> Self {
        Self::Compound(val)
    }
}

impl From<Vec<Value>> for Value {
    fn from(val: Vec<Value>) -> Self {
        Self::List(val)
    }
}

impl From<&[Value]> for Value {
    fn from(val: &[Value]) -> Self {
        Self::List(val.to_vec())
    }
}

impl From<Vec<i32>> for Value {
    fn from(val: Vec<i32>) -> Self {
        Self::IntArray(val)
    }
}

impl From<&[i32]> for Value {
    fn from(val: &[i32]) -> Self {
        Self::IntArray(val.to_vec())
    }
}

impl From<Vec<i64>> for Value {
    fn from(val: Vec<i64>) -> Self {
        Self::LongArray(val)
    }
}

impl From<&[i64]> for Value {
    fn from(val: &[i64]) -> Self {
        Self::LongArray(val.to_vec())
    }
}
