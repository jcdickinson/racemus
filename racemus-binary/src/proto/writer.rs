use crate::{BinaryWriter, BinaryWriterInsertion, Error, ErrorKind};
use async_std::io::Write;

const MAX_LEN: usize = (std::i32::MAX as u32) as usize;

impl<W: Write + Unpin> BinaryWriter<W> {
    #[inline]
    pub(crate) fn len_var_i32(&mut self, val: usize) -> Result<&mut Self, Error> {
        if val > MAX_LEN {
            return Err(ErrorKind::LengthTooLarge.into());
        }
        self.var_i32(val as i32)
    }

    #[inline]
    pub(crate) fn arr_u8(&mut self, val: &[u8]) -> Result<&mut Self, Error> {
        self.len_var_i32(val.len())?;
        self.raw_buffer(val)
    }

    #[inline]
    pub(crate) fn arr_char(&mut self, val: &str) -> Result<&mut Self, Error> {
        self.arr_u8(val.as_bytes())
    }

    pub(crate) fn insert_len_var_i32(
        &mut self,
        insertion: BinaryWriterInsertion,
    ) -> Result<&mut Self, Error> {
        let len = self.bytes_after_insertion(&insertion);
        if len > MAX_LEN {
            return Err(ErrorKind::LengthTooLarge.into());
        }
        self.insert_var_i32(insertion, len as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    pub fn binary_writer_insert_len_var_i32() -> Result<(), Error> {
        let mut writer = make_writer();

        let pre = writer.create_insertion();
        writer.raw_buffer(b"1234" as &[u8])?;
        writer.insert_len_var_i32(pre)?;

        let buf = make_buffer(writer);
        assert_eq!(buf, b"\x041234");

        Ok(())
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

    raw_write_tests!(
        binary_writer_arr_char, "test-data/arr-char-1.in", w => w
            .arr_char("this is a string test 🎉✨")?
            .arr_char("this is a string test1 🎉✨")?;
        binary_writer_arr_u8, "test-data/arr-u8-1.in", w => w
            .arr_u8(b"12345")?
            .arr_u8(b"567890")?;
    );
}
