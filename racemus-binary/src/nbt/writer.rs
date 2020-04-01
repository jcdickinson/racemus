use crate::{nbt::Value, BinaryWriter, Error, ErrorKind};
use async_std::io::Write;

const MAX_LEN: usize = (std::i32::MAX as u32) as usize;
const MAX_NAME_LEN: usize = (std::i16::MAX as u16) as usize;

fn type_id_for(value: &Value) -> u8 {
    match value {
        Value::Byte(_) => 0x01,
        Value::Short(_) => 0x02,
        Value::Int(_) => 0x03,
        Value::Long(_) => 0x04,
        Value::Float(_) => 0x05,
        Value::Double(_) => 0x06,
        Value::ByteArray(_) => 0x07,
        Value::String(_) => 0x08,
        Value::List(_) => 0x09,
        Value::Compound(_) => 0x0a,
        Value::IntArray(_) => 0x0b,
        Value::LongArray(_) => 0x0c,
    }
}

impl<W: Write + Unpin> BinaryWriter<W> {
    fn nbt_inner(&mut self, value: &Value) -> Result<&mut Self, Error> {
        match value {
            Value::Byte(b) => self.fix_i8(*b),
            Value::Short(s) => self.fix_i16(*s),
            Value::Int(i) => self.fix_i32(*i),
            Value::Long(l) => self.fix_i64(*l),
            Value::Float(f) => self.fix_f32(*f),
            Value::Double(d) => self.fix_f64(*d),
            Value::ByteArray(b) => {
                if b.len() > MAX_LEN {
                    return Err(ErrorKind::LengthTooLarge.into());
                }
                self.fix_i32((b.len() as u32) as i32)?.raw_buffer(&b)
            }
            Value::String(s) => {
                let cesu = cesu8::to_java_cesu8(&s);
                if cesu.len() > MAX_NAME_LEN {
                    return Err(ErrorKind::LengthTooLarge.into());
                }
                self.fix_i16((cesu.len() as u16) as i16)?.raw_buffer(&cesu)
            }
            Value::List(l) => {
                if l.len() == 0 {
                    self.fix_u8(0)?.fix_i32(0)
                } else if l.len() > MAX_LEN {
                    Err(ErrorKind::LengthTooLarge.into())
                } else {
                    let type_id = type_id_for(&l[0]);
                    self.fix_u8(type_id)?.fix_i32((l.len() as u32) as i32)?;
                    for v in l.as_ref() {
                        if type_id_for(v) != type_id {
                            return Err(ErrorKind::InvalidNbt.into());
                        }
                        self.nbt_inner(v)?;
                    }
                    Ok(self)
                }
            }
            Value::Compound(m) => {
                for (n, ref v) in m {
                    self.nbt(n, v)?;
                }
                self.fix_u8(0)
            }
            Value::IntArray(ia) => {
                if ia.len() > MAX_LEN {
                    return Err(ErrorKind::LengthTooLarge.into());
                }
                for i in ia.iter() {
                    self.fix_i32(*i)?;
                }
                Ok(self)
            }
            Value::LongArray(la) => {
                if la.len() > MAX_LEN {
                    return Err(ErrorKind::LengthTooLarge.into());
                }
                for l in la.iter() {
                    self.fix_i64(*l)?;
                }
                Ok(self)
            }
        }
    }

    pub fn nbt(&mut self, name: &str, value: &Value) -> Result<&mut Self, Error> {
        if name.len() > MAX_NAME_LEN {
            return Err(ErrorKind::LengthTooLarge.into());
        }

        let name = cesu8::to_java_cesu8(name);

        self.fix_u8(type_id_for(value))?
            .fix_u16(name.len() as u16)?
            .raw_buffer(&name)?
            .nbt_inner(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    // Intentionally unused code as a reminder to create tests.

    macro_rules! raw_write_tests {
        ($($name:ident, $expected:expr, $writer:ident => $expr:expr;)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let mut $writer = make_writer();
                    $expr;
                    let buf = make_buffer($writer);
                    assert_eq!(buf, include_bytes!($expected) as &[u8]);
                    Ok(())
                }
            )*
        }
    }
}
