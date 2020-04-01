use crate::{nbt::Value, BinaryReader, Error, ErrorKind};
use async_std::io::Read;
use std::{collections::HashMap, marker::Unpin, sync::Arc};

enum StackState {
    Compound(Arc<str>, HashMap<Arc<str>, Value>),
    List(Arc<str>, Vec<Value>, u8, usize),
}

fn push_value(
    stack: &mut Vec<StackState>,
    name: Arc<str>,
    value: Value,
) -> Option<(Arc<str>, Value)> {
    let peek = stack.pop();
    match peek {
        None => Some((name, value)),
        Some(peek) => match peek {
            StackState::Compound(peek_name, mut peek_map) => {
                peek_map.insert(name, value);
                stack.push(StackState::Compound(peek_name, peek_map));
                None
            }
            StackState::List(peek_name, mut peek_list, peek_type_id, peek_size) => {
                peek_list.push(value);
                if peek_list.len() == peek_size {
                    Some((peek_name, Value::List(peek_list[..].into())))
                } else {
                    stack.push(StackState::List(
                        peek_name,
                        peek_list,
                        peek_type_id,
                        peek_size,
                    ));
                    None
                }
            }
        },
    }
}

impl<R: Read + Unpin> BinaryReader<R> {
    async fn length_fix_i16(&mut self) -> Result<usize, Error> {
        let length = self.fix_i16().await?;
        if length < 0 {
            return Err(ErrorKind::InvalidLengthPrefix.into());
        }
        Ok(length as usize)
    }

    async fn length_fix_i32(&mut self) -> Result<usize, Error> {
        let length = self.fix_i32().await?;
        if length < 0 {
            return Err(ErrorKind::InvalidLengthPrefix.into());
        }
        Ok(length as usize)
    }

    async fn str_fix_i16(&mut self) -> Result<Arc<str>, Error> {
        let length = self.length_fix_i16().await?;
        let result = self.str(length as usize).await?;
        Ok(result)
    }

    async fn str(&mut self, count: usize) -> Result<Arc<str>, Error> {
        let name = self.data(count).await?;
        let name = cesu8::from_java_cesu8(name)?;
        let name = name.into_owned()[..].into();
        self.consume(count);
        Ok(name)
    }

    async fn tag_name(&mut self) -> Result<(u8, Arc<str>), Error> {
        let type_id = self.fix_u8().await?;
        if type_id == 0x00 {
            Ok((0x00, ""[..].into()))
        } else {
            Ok((type_id, self.str_fix_i16().await?))
        }
    }

    async fn tag(&mut self, stack: &mut Vec<StackState>) -> Result<(u8, Arc<str>), Error> {
        if let Some(peek) = stack.pop() {
            let result = match peek {
                StackState::List(_, _, type_id, _) => (type_id, ""[..].into()),
                StackState::Compound(_, _) => self.tag_name().await?,
            };
            stack.push(peek);
            Ok(result)
        } else {
            let result = self.tag_name().await?;
            Ok(result)
        }
    }

    pub async fn nbt(&mut self) -> Result<(Arc<str>, Value), Error> {
        let mut stack = Vec::with_capacity(2);

        loop {
            let (type_id, name) = self.tag(&mut stack).await?;

            let mut result = match type_id {
                0x09 => {
                    let type_id = self.fix_u8().await?;
                    let size = self.length_fix_i32().await?;
                    if size == 0 {
                        let arr: Vec<Value> = Vec::with_capacity(0);
                        Some((name, Value::List(arr[..].into())))
                    } else {
                        stack.push(StackState::List(
                            name,
                            Vec::with_capacity(size),
                            type_id,
                            size,
                        ));
                        None
                    }
                }
                0x0a => {
                    stack.push(StackState::Compound(name, HashMap::new()));
                    None
                }
                0x00 => {
                    if let Some(top) = stack.pop() {
                        match top {
                            StackState::Compound(name, map) => Some((name, Value::Compound(map))),
                            _ => return Err(ErrorKind::InvalidNbt.into()),
                        }
                    } else {
                        return Err(ErrorKind::InvalidNbt.into());
                    }
                }
                0x01 => Some((name, Value::Byte(self.fix_i8().await?))),
                0x02 => Some((name, Value::Short(self.fix_i16().await?))),
                0x03 => Some((name, Value::Int(self.fix_i32().await?))),
                0x04 => Some((name, Value::Long(self.fix_i64().await?))),
                0x05 => Some((name, Value::Float(self.fix_f32().await?))),
                0x06 => Some((name, Value::Double(self.fix_f64().await?))),
                0x07 => {
                    let count = self.length_fix_i32().await?;
                    let data = Value::ByteArray(self.data(count).await?.into());
                    self.consume(count);
                    Some((name, data))
                }
                0x08 => Some((name, Value::String(self.str_fix_i16().await?))),
                0x0b => {
                    let len = self.length_fix_i32().await?;
                    let mut vec = Vec::with_capacity(len);
                    for _ in 0..len {
                        vec.push(self.fix_i32().await?)
                    }
                    Some((name, Value::IntArray(vec[..].into())))
                }
                0x0c => {
                    let len = self.length_fix_i32().await?;
                    let mut vec = Vec::with_capacity(len);
                    for _ in 0..len {
                        vec.push(self.fix_i64().await?)
                    }
                    Some((name, Value::LongArray(vec[..].into())))
                }
                _ => return Err(ErrorKind::InvalidNbt.into()),
            };

            while let Some(current) = result {
                if stack.len() == 0 {
                    return Ok(current);
                }
                result = push_value(&mut stack, current.0, current.1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    macro_rules! raw_read_tests {
        ($($name:ident, $input:expr, $reader:ident => { $($expr:expr, $expected:expr;)* };)*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut $reader = $crate::tests::make_reader(include_bytes!($input) as &[u8]);
                    $reader.with_size(None);
                    $({
                        let actual = block_on($expr)?;
                        assert_eq!(actual, $expected);
                    })*
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        read_nbt, "test-data/hello-world.in", r => {
            r.nbt(), ("hello world"[..].into(),
                crate::nbt_compound!{
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
                }
            );
        };
    );
}
