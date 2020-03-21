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
            if let Some(current_len) = self.current_len {
                if current_len < SIZE {
                    return Err(ErrorKind::ReadPastPacket.into());
                }
            }
            if self.buffer.available_data() < SIZE {
                self.fill(SIZE).await?
            }
            let result = <$type>::from_be_bytes(self.buffer.data()[0..SIZE].try_into().unwrap());
            if let Some(current_len) = self.current_len.as_mut() {
                *current_len -= SIZE;
            }
            self.buffer.consume(SIZE);
            Ok(result)
        }
    };
}

impl<R: Read + Unpin> BinaryReader<R> {
    #[cfg(not(test))]
    const BUFFER_INIT: usize = 1024;
    #[cfg(not(test))]
    const BUFFER_GROW: usize = 4096;
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
        // circular won't allow you to edit data that is already committed.
        self.cipher = Some(cipher);
        self
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn current_len(&self) -> Option<usize> {
        self.current_len
    }

    pub(crate) async fn fill(&mut self, size: usize) -> Result<(), Error> {
        if size > self.buffer.available_space() {
            let size = size - self.buffer.available_data();
            // Size it to BUFFER_GROW increments
            let size = (size + Self::BUFFER_GROW - 1) / Self::BUFFER_GROW;
            let size = size * Self::BUFFER_GROW;

            self.buffer.grow(size);
        }
        while self.buffer.available_data() < size {
            let n = match self.reader.read(&mut self.buffer.space()).await {
                Ok(r) => r,
                Err(e) => return Err(Box::new(e).into()),
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

    async fn consume_remainder(&mut self) -> Result<(), Error> {
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
                        Err(e) => return Err(Box::new(e).into()),
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
    #[allow(dead_code)]
    pub(crate) async fn with_size(
        &mut self,
        size: Option<usize>,
    ) -> Result<(), Error> {
        self.consume_remainder().await?;
        self.current_len = size;
        Ok(())
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
    pub fn test_binary_reader_encryption() -> Result<(), Error> {
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

    macro_rules! raw_read_tests {
        ($($name:ident, $input:literal: $reader:ident => { $($expr:expr, $expected:expr),* }),*) => {
            $(
                #[test]
                pub fn $name() -> Result<(), Error> {
                    let mut $reader = make_reader($input as &[u8]);
                    $({
                        assert_eq!(block_on($expr)?, $expected);
                    })*
                    Ok(())
                }
            )*
        }
    }

    raw_read_tests!(
        read_fix_u, b"\x15\x15\x26\x15\x26\x37\x49\x15\x26\x37\x49\x50\x15\x26\x37": r => {
            r.fix_u8(), 0x15,
            r.fix_u16(), 0x1526,
            r.fix_u32(), 0x1526_3749,
            r.fix_u64(), 0x1526_3749_5015_2637
        },
        read_fix_i, b"\xeb\xea\xda\xea\xd9\xc8\xb7\xea\xd9\xc8\xb6\xaf\xea\xd9\xc9": r => {
            r.fix_i8(), -0x15,
            r.fix_i16(), -0x1526,
            r.fix_i32(), -0x1526_3749,
            r.fix_i64(), -0x1526_3749_5015_2637
        },
        read_fix_f, b"\x40\x2d\xf8\x54\x40\x05\xbf\x0a\x8b\x14\x57\x69": r => {
            r.fix_f32(), std::f32::consts::E,
            r.fix_f64(), std::f64::consts::E
        },
        read_var_i16, b"\x00\x01\x02\x7f\xff\x01\xff\xff\x01\xff\xff\x03\x80\x80\x02": r => {
            r.var_i16(), 0x0000,
            r.var_i16(), 0x0001,
            r.var_i16(), 0x0002,
            r.var_i16(), 0x007f,
            r.var_i16(), 0x00ff,
            r.var_i16(), 0x7fff,
            r.var_i16(), -0x0001,
            r.var_i16(), -0x8000
        },
        read_var_i32, b"\x00\x01\x02\x7f\xff\x01\xff\xff\xff\xff\x07\xff\xff\xff\xff\x0f\x80\x80\x80\x80\x08": r => {
            r.var_i32(), 0x0000_0000,
            r.var_i32(), 0x0000_0001,
            r.var_i32(), 0x0000_0002,
            r.var_i32(), 0x0000_007f,
            r.var_i32(), 0x0000_00ff,
            r.var_i32(), 0x7fff_ffff,
            r.var_i32(), -0x0000_0001,
            r.var_i32(), -0x8000_0000
        },
        read_var_i64, b"\x00\x01\x02\x7f\xff\x01\xff\xff\xff\xff\xff\xff\xff\xff\x7f\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01\x80\x80\x80\x80\x80\x80\x80\x80\x80\x01": r => {
            r.var_i64(), 0x0000_0000_0000_0000,
            r.var_i64(), 0x0000_0000_0000_0001,
            r.var_i64(), 0x0000_0000_0000_0002,
            r.var_i64(), 0x0000_0000_0000_007f,
            r.var_i64(), 0x0000_0000_0000_00ff,
            r.var_i64(), 0x7fff_ffff_ffff_ffff,
            r.var_i64(), -0x0000_0000_0000_0001,
            r.var_i64(), -0x8000_0000_0000_0000
        },
        read_var_u16, b"\x00\x01\x02\x7f\xff\x01\xff\xff\x01\xff\xff\x03": r => {
            r.var_u16(), 0x0000,
            r.var_u16(), 0x0001,
            r.var_u16(), 0x0002,
            r.var_u16(), 0x007f,
            r.var_u16(), 0x00ff,
            r.var_u16(), 0x7fff,
            r.var_u16(), 0xffff
        },
        read_var_u32, b"\x00\x01\x02\x7f\xff\x01\xff\xff\xff\xff\x07\xff\xff\xff\xff\x0f": r => {
            r.var_u32(), 0x0000_0000,
            r.var_u32(), 0x0000_0001,
            r.var_u32(), 0x0000_0002,
            r.var_u32(), 0x0000_007f,
            r.var_u32(), 0x0000_00ff,
            r.var_u32(), 0x7fff_ffff,
            r.var_u32(), 0xffff_ffff
        },
        read_var_u64, b"\x00\x01\x02\x7f\xff\x01\xff\xff\xff\xff\xff\xff\xff\xff\x7f\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01": r => {
            r.var_u64(), 0x0000_0000_0000_0000,
            r.var_u64(), 0x0000_0000_0000_0001,
            r.var_u64(), 0x0000_0000_0000_0002,
            r.var_u64(), 0x0000_0000_0000_007f,
            r.var_u64(), 0x0000_0000_0000_00ff,
            r.var_u64(), 0x7fff_ffff_ffff_ffff,
            r.var_u64(), 0xffff_ffff_ffff_ffff
        }
    );
}
