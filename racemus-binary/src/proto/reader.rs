use crate::{BinaryReader, Error, ErrorKind};
use async_std::io::Read;
use std::{marker::Unpin, sync::Arc};

impl<R: Read + Unpin> BinaryReader<R> {
    #[inline]
    pub(crate) async fn len_var_i32(&mut self, max: Option<usize>) -> Result<usize, Error> {
        let count = self.var_i32().await?;
        if count < 0 {
            return Err(ErrorKind::InvalidLengthPrefix.into());
        }

        let count = count as usize;
        if let Some(max) = max {
            if count > max {
                return Err(ErrorKind::InvalidLengthPrefix.into());
            }
        }

        self.validate_length(count)?;
        Ok(count)
    }

    #[inline]
    pub(crate) async fn packet_header(&mut self) -> Result<i32, Error> {
        self.consume_remainder().await?;

        self.with_size(None); // Ensure length can be read
        let count = self.len_var_i32(None).await?;
        self.with_size(Some(count));

        let packet_id = self.var_i32().await?;
        Ok(packet_id)
    }

    #[inline]
    async fn raw_arr_u8(&mut self, max: Option<usize>) -> Result<&[u8], Error> {
        let count = self.len_var_i32(max).await?;
        let data = self.data(count).await?;
        Ok(data)
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn arr_u8(&mut self, max: Option<usize>) -> Result<Arc<[u8]>, Error> {
        let data = self.raw_arr_u8(max).await?;
        let data: Arc<[u8]> = data.into();
        self.consume(data.len());
        Ok(data)
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) async fn arr_char(&mut self, max: Option<usize>) -> Result<Arc<str>, Error> {
        let raw = self.raw_arr_u8(max).await?;
        let count = raw.len();
        match std::str::from_utf8(&raw) {
            Ok(data) => {
                let data: Arc<str> = data.into();
                self.consume(count);
                Ok(data)
            }
            Err(e) => Err(ErrorKind::InvalidString(e).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    pub fn binary_reader_len_var_i32_happy_path() -> Result<(), Error> {
        let mut reader = make_reader(b"\x2f");

        assert_eq!(block_on(reader.len_var_i32(None))?, 0x2f);

        Ok(())
    }

    #[test]
    pub fn binary_reader_len_var_i32_field_long() -> Result<(), Error> {
        let mut reader = make_reader(b"\x7f");
        match block_on(reader.len_var_i32(Some(5))) {
            Ok(r) => assert_ne!(r, r),
            Err(e) => match e.kind() {
                ErrorKind::InvalidLengthPrefix => {}
                _ => return Err(e),
            },
        }

        Ok(())
    }

    #[test]
    pub fn binary_reader_len_var_i32_packet_long() -> Result<(), Error> {
        let mut reader = make_reader(b"\x7f");
        reader.with_size(Some(5));
        match block_on(reader.len_var_i32(None)) {
            Ok(r) => assert_ne!(r, r),
            Err(e) => match e.kind() {
                ErrorKind::ReadPastPacket => {}
                _ => return Err(e),
            },
        }

        Ok(())
    }

    #[test]
    pub fn binary_reader_packet_header() -> Result<(), Error> {
        let mut reader = make_reader(b"\x03\x0123\x01\x15");
        assert_eq!(block_on(reader.packet_header())?, 1);
        assert_eq!(block_on(reader.packet_header())?, 0x15);

        Ok(())
    }

    macro_rules! raw_read_tests {
        ($($name:ident, $input:expr, $reader:ident => { $($expr:expr, $expected:expr;)* };)*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut $reader = make_reader(include_bytes!($input) as &[u8]);
                    $({
                        assert_eq!(block_on($expr)?, $expected);
                    })*
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        binary_reader_arr_char, "test-data/arr-char-1.in", r => {
            r.arr_char(None), "this is a string test 🎉✨".into();
            r.arr_char(None), "this is a string test1 🎉✨".into();
        };
        binary_reader_arr_u8, "test-data/arr-u8-1.in", r => {
            r.arr_u8(None), (b"12345" as &[u8]).into();
            r.arr_u8(None), (b"567890" as &[u8]).into();
        };
    );
}
