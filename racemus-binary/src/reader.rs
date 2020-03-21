use crate::{AesCfb8, Error, ErrorKind};
use async_std::io::{prelude::*, Read};
use cfb8::stream_cipher::StreamCipher;
use circular::Buffer;
use std::{convert::TryInto, marker::Unpin};

pub struct BinaryReader<R: Read + Unpin> {
    buffer: Buffer,
    current_len: Option<usize>,
    reader: R,
    cipher: Option<AesCfb8>,
}

macro_rules! build_read_varint {
    ($name:ident, $type:ty) => {
        #[inline]
        #[allow(dead_code)]
        pub(crate) async fn $name(&mut self) -> Result<$type, Error> {
            const SIZE: usize = std::mem::size_of::<$type>() * 8;
            let mut res: u64 = 0;
            let mut shift: usize = 0;
            loop {
                let byte = self.fix_u8().await?;
                res |= ((byte as u64) & 0b0111_1111) << shift;
                if (byte & 0b1000_0000) == 0 {
                    return Ok(res as $type);
                }
                shift += 7;
                if shift > SIZE {
                    return Err(ErrorKind::InvalidVarint.into());
                }
            }
        }
    };
}
macro_rules! build_read_fixnum {
    ($name:ident, $type:ty) => {
        #[inline]
        #[allow(dead_code)]
        pub(crate) async fn $name(&mut self) -> Result<$type, Error> {
            const SIZE: usize = std::mem::size_of::<$type>();
            let data = self.data(SIZE).await?;
            let result = <$type>::from_be_bytes(data.try_into().unwrap());
            self.consume(SIZE);
            Ok(result)
        }
    };
}

impl<R: Read + Unpin> BinaryReader<R> {
    #[cfg(not(test))]
    const BUFFER_INIT: usize = 1024;
    #[cfg(not(test))]
    const BUFFER_GROW: usize = 4096;
    // Small values are intentionally used in tests to ensure that
    // streaming is working
    #[cfg(test)]
    const BUFFER_INIT: usize = 1;
    #[cfg(test)]
    const BUFFER_GROW: usize = 2;
    pub fn new(reader: R) -> Self {
        Self {
            buffer: Buffer::with_capacity(Self::BUFFER_INIT),
            current_len: None,
            reader,
            cipher: None,
        }
    }

    #[inline]
    pub fn decrypt(&mut self, cipher: AesCfb8) -> &mut Self {
        // We don't need to decrypt the data retroactively because the
        // encryption negotiation is lock-step. This is fortunate because
        // circular wouldn't allow us to edit committed data anyway.
        self.cipher = Some(cipher);
        self
    }

    #[inline]
    pub(crate) fn validate_length(&self, count: usize) -> Result<(), Error> {
        if let Some(current_len) = self.current_len {
            if current_len < count {
                return Err(ErrorKind::ReadPastPacket.into());
            }
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn consume(&mut self, count: usize) {
        if let Some(current_len) = self.current_len.as_mut() {
            *current_len -= count;
        }
        self.buffer.consume_noshift(count);
    }

    #[inline]
    pub(crate) async fn data(&mut self, count: usize) -> Result<&[u8], Error> {
        self.validate_length(count)?;
        if self.buffer.available_data() < count {
            self.fill(count).await?
        }
        Ok(&self.buffer.data()[0..count])
    }

    async fn fill(&mut self, count: usize) -> Result<(), Error> {
        if count > self.buffer.available_space() {
            self.buffer.shift();
            if count > self.buffer.available_space() {
                let grow = count;
                // Size it to BUFFER_GROW increments
                let grow = (grow + Self::BUFFER_GROW - 1) / Self::BUFFER_GROW;
                let grow = grow * Self::BUFFER_GROW;

                self.buffer.grow(grow);
            }
        }
        while self.buffer.available_data() < count {
            let n = match self.reader.read(&mut self.buffer.space()).await {
                Ok(r) => r,
                Err(e) => return Err(ErrorKind::IOError(e).into()),
            };
            if n == 0 {
                return Err(ErrorKind::EndOfData.into());
            }

            if let Some(cipher) = self.cipher.as_mut() {
                cipher.decrypt(&mut self.buffer.space()[0..n]);
            }
            self.buffer.fill(n);
        }
        Ok(())
    }

    pub(crate) async fn consume_remainder(&mut self) -> Result<(), Error> {
        if let Some(current_len) = self.current_len.as_mut() {
            if *current_len != 0 {
                let remove = std::cmp::min(*current_len, self.buffer.available_data());
                *current_len -= remove;
                self.buffer.consume(remove);
                while *current_len != 0 {
                    // We are not decrypting, so don't overfill.
                    let remove = std::cmp::min(*current_len, self.buffer.available_space());
                    let remove = match self.reader.read(&mut self.buffer.space()[0..remove]).await {
                        Ok(r) => r,
                        Err(e) => return Err(ErrorKind::IOError(e).into()),
                    };
                    if remove == 0 {
                        return Err(ErrorKind::EndOfData.into());
                    }
                    *current_len -= remove;
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn with_size(&mut self, count: Option<usize>) {
        self.current_len = count;
    }

    build_read_fixnum!(fix_i8, i8);
    build_read_fixnum!(fix_i16, i16);
    build_read_fixnum!(fix_i32, i32);
    build_read_fixnum!(fix_i64, i64);

    build_read_varint!(var_u16, u16);
    build_read_varint!(var_u32, u32);
    build_read_varint!(var_u64, u64);

    build_read_varint!(var_i16, i16);
    build_read_varint!(var_i32, i32);
    build_read_varint!(var_i64, i64);

    build_read_fixnum!(fix_u8, u8);
    build_read_fixnum!(fix_u16, u16);
    build_read_fixnum!(fix_u32, u32);
    build_read_fixnum!(fix_u64, u64);

    build_read_fixnum!(fix_f32, f32);
    build_read_fixnum!(fix_f64, f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use cfb8::stream_cipher::NewStreamCipher;

    #[test]
    pub fn binary_reader_encryption() -> Result<(), Error> {
        let mut reader = make_reader(b"\x2f\x57\xb5\x42");

        reader.decrypt(
            crate::AesCfb8::new_var(b"0234567890123456" as &[u8], b"0234567890123456" as &[u8])
                .unwrap(),
        );

        assert_eq!(block_on(reader.fix_u8())?, 1);
        assert_eq!(block_on(reader.fix_u8())?, 10);
        assert_eq!(block_on(reader.fix_u8())?, 1);
        assert_eq!(block_on(reader.fix_u8())?, 10);

        Ok(())
    }

    #[test]
    pub fn binary_reader_with_size_read_incomplete() -> Result<(), Error> {
        let mut reader = make_reader(b"1234\x15\x26");

        reader.with_size(Some(4));
        // Value is expected to be skipped
        block_on(reader.consume_remainder())?;

        reader.with_size(Some(1));
        assert_eq!(block_on(reader.fix_u8())?, 0x15);
        block_on(reader.consume_remainder())?;

        reader.with_size(Some(1));
        assert_eq!(block_on(reader.fix_u8())?, 0x26);
        block_on(reader.consume_remainder())?;

        Ok(())
    }

    #[test]
    pub fn binary_reader_with_size_readpast() -> Result<(), Error> {
        let mut reader = make_reader(b"\x15\x26");

        reader.with_size(Some(1));
        match block_on(reader.data(2)) {
            Ok(_) => panic!("expected error"),
            Err(e) => match e.kind() {
                ErrorKind::ReadPastPacket => {}
                _ => return Err(e),
            },
        }

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
        binary_reader_fix_unsigned, "test-data/fix-unsigned-1.in", r => {
            r.fix_u8(), 0x15;
            r.fix_u16(), 0x1526;
            r.fix_u32(), 0x1526_3749;
            r.fix_u64(), 0x1526_3749_5015_2637;
        };
        binary_reader_fix_signed, "test-data/fix-signed-1.in", r => {
            r.fix_i8(), -0x15;
            r.fix_i16(), -0x1526;
            r.fix_i32(), -0x1526_3749;
            r.fix_i64(), -0x1526_3749_5015_2637;
        };
        binary_reader_fix_float, "test-data/fix-float-1.in", r => {
            r.fix_f32(), std::f32::consts::E;
            r.fix_f64(), std::f64::consts::E;
        };
        binary_reader_var_i16, "test-data/var-signed-16-1.in", r => {
            r.var_i16(), 0x0000;
            r.var_i16(), 0x0001;
            r.var_i16(), 0x0002;
            r.var_i16(), 0x007f;
            r.var_i16(), 0x00ff;
            r.var_i16(), 0x7fff;
            r.var_i16(), -0x0001;
            r.var_i16(), -0x8000;
        };
        binary_reader_var_i32, "test-data/var-signed-32-1.in", r => {
            r.var_i32(), 0x0000_0000;
            r.var_i32(), 0x0000_0001;
            r.var_i32(), 0x0000_0002;
            r.var_i32(), 0x0000_007f;
            r.var_i32(), 0x0000_00ff;
            r.var_i32(), 0x7fff_ffff;
            r.var_i32(), -0x0000_0001;
            r.var_i32(), -0x8000_0000;
        };
        binary_reader_var_i64, "test-data/var-signed-64-1.in", r => {
            r.var_i64(), 0x0000_0000_0000_0000;
            r.var_i64(), 0x0000_0000_0000_0001;
            r.var_i64(), 0x0000_0000_0000_0002;
            r.var_i64(), 0x0000_0000_0000_007f;
            r.var_i64(), 0x0000_0000_0000_00ff;
            r.var_i64(), 0x7fff_ffff_ffff_ffff;
            r.var_i64(), -0x0000_0000_0000_0001;
            r.var_i64(), -0x8000_0000_0000_0000;
        };
        binary_reader_var_u16, "test-data/var-unsigned-16-1.in", r => {
            r.var_u16(), 0x0000;
            r.var_u16(), 0x0001;
            r.var_u16(), 0x0002;
            r.var_u16(), 0x007f;
            r.var_u16(), 0x00ff;
            r.var_u16(), 0x7fff;
            r.var_u16(), 0xffff;
        };
        binary_reader_var_u32, "test-data/var-unsigned-32-1.in", r => {
            r.var_u32(), 0x0000_0000;
            r.var_u32(), 0x0000_0001;
            r.var_u32(), 0x0000_0002;
            r.var_u32(), 0x0000_007f;
            r.var_u32(), 0x0000_00ff;
            r.var_u32(), 0x7fff_ffff;
            r.var_u32(), 0xffff_ffff;
        };
        binary_reader_var_u64, "test-data/var-unsigned-64-1.in", r => {
            r.var_u64(), 0x0000_0000_0000_0000;
            r.var_u64(), 0x0000_0000_0000_0001;
            r.var_u64(), 0x0000_0000_0000_0002;
            r.var_u64(), 0x0000_0000_0000_007f;
            r.var_u64(), 0x0000_0000_0000_00ff;
            r.var_u64(), 0x7fff_ffff_ffff_ffff;
            r.var_u64(), 0xffff_ffff_ffff_ffff;
        };
    );
}
