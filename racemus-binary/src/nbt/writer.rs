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
                self.fix_i32(ia.len() as i32)?;
                for i in ia.iter() {
                    self.fix_i32(*i)?;
                }
                Ok(self)
            }
            Value::LongArray(la) => {
                if la.len() > MAX_LEN {
                    return Err(ErrorKind::LengthTooLarge.into());
                }
                self.fix_i32(la.len() as i32)?;
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

    // HashMap is not deterministic, so we have to use the read functions as a
    // source of truth for compound with multiple values. The read functions are
    // in-turn tested against a nbt file generated with an external tool.

    macro_rules! identity_tests {
        ($($name:ident, $nbt_name:literal => $expected:expr;)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let mut writer = make_writer();
                    writer.nbt(&$nbt_name, &$expected)?;
                    let buf = make_buffer(writer);
                    let mut reader = make_reader(&buf);
                    let actual = block_on(reader.nbt())?;
                    assert_eq!(actual, ($nbt_name[..].into(), $expected));
                    Ok(())
                }
            )*
        }
    }

    identity_tests! {
        binary_writer_nbt_hello_world, "hello world" => crate::nbt_compound!{
            "byte" => crate::nbt_byte!(127),
            "short" => crate::nbt_short!(16383),
            "int" => crate::nbt_int!(1073741823),
            "long" => crate::nbt_long!(4611686018427387903),
            "float" => crate::nbt_float!(123.456),
            "double" => crate::nbt_double!(12.456f64),
            "bytearr" => crate::nbt_byte_array![18, 52, 86, 120, 144, 171, 205],
            "str" => crate::nbt_string!("strtest"),
            "intarr" => crate::nbt_int_array![1234, 222, 333, 444, 555, 666, 777, 888, 999, 1111],
            "longarr" => crate::nbt_long_array![11112, 112334, 412412, 123412],
            "lst" => crate::nbt_list![
                crate::nbt_string!("str1"),
                crate::nbt_string!("str2")
            ],
            "comp" => crate::nbt_compound!{
                "byte" => crate::nbt_byte!(123),
                "short" => crate::nbt_short!(456)
            }
        };
    }

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

    raw_write_tests! {
        binary_writer_nbt_byte, "test-data/nbt-byte-1.in", w =>
            w.nbt("byte", &crate::nbt_byte!(123))?;
        binary_writer_nbt_short, "test-data/nbt-short-1.in", w =>
            w.nbt("short", &crate::nbt_short!(-6141))?;
        binary_writer_nbt_int, "test-data/nbt-int-1.in", w =>
            w.nbt("int", &crate::nbt_int!(14808325))?;
        binary_writer_nbt_long, "test-data/nbt-long-1.in", w =>
            w.nbt("long", &crate::nbt_long!(152134054404865))?;
        binary_writer_nbt_float, "test-data/nbt-float-1.in", w =>
            w.nbt("float", &crate::nbt_float!(1247623.5))?;
        binary_writer_nbt_double, "test-data/nbt-double-1.in", w =>
            w.nbt("double", &crate::nbt_double!(123455678.12345))?;
        binary_writer_nbt_byte_array, "test-data/nbt-byte-array-1.in", w =>
            w.nbt("barray", &crate::nbt_byte_array![1, 2, 3, 4])?;
        binary_writer_nbt_string, "test-data/nbt-string-1.in", w =>
            w.nbt("string", &crate::nbt_string!("this is a string test 🎉✨"))?;
        binary_writer_nbt_list_byte, "test-data/nbt-list-byte-1.in", w =>
            w.nbt("list", &crate::nbt_list![
                crate::nbt_byte!(1),
                crate::nbt_byte!(2),
                crate::nbt_byte!(3),
                crate::nbt_byte!(4)
            ])?;
        binary_writer_nbt_list_empty, "test-data/nbt-list-empty-1.in", w =>
            w.nbt("list", &crate::nbt_list![])?;
        binary_writer_nbt_compound_single, "test-data/nbt-compound-single-1.in", w =>
            w.nbt("comp", &crate::nbt_compound!{
                "byte" => crate::nbt_byte!(124)
            })?;
        binary_writer_nbt_compound_empty, "test-data/nbt-compound-empty-1.in", w =>
            w.nbt("comp", &crate::nbt_compound!{})?;
        binary_writer_nbt_int_array, "test-data/nbt-int-array-1.in", w =>
            w.nbt("iarray", &crate::nbt_int_array![1, 2, 3, 4])?;
        binary_writer_nbt_long_array, "test-data/nbt-long-array-1.in", w =>
            w.nbt("larray", &crate::nbt_long_array![1, 2, 3, 4])?;
    }
}
