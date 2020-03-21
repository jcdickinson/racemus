use crate::AesCfb8;
use crate::{Error, ErrorKind};
use async_std::io::{prelude::*, Write};
use cfb8::stream_cipher::StreamCipher;
use std::marker::Unpin;
use std::ops::Range;

pub trait StructuredWriter<W: Write + Unpin, T> {
    fn structure(&mut self, val: &T) -> Result<&mut Self, Error>;
}

pub struct BinaryWriter<W: Write + Unpin> {
    order: Vec<Option<Range<usize>>>,
    buffer: Vec<u8>,
    writer: W,
    cipher: Option<AesCfb8>,
}

macro_rules! build_write_varint {
    ($name:ident, $type:ty, $unsigned:ty) => {
        #[inline]
        #[allow(dead_code)]
        pub(crate) fn $name(&mut self, val: $type) -> Result<&mut Self, Error> {
            const SIZE: usize = std::mem::size_of::<$type>();
            const SHIFT: usize = 7;
            const BITSIZE: usize = SIZE * 8;
            const MAX_BYTES: usize = (BITSIZE + SHIFT - 1) / SHIFT;

            let mut val = val as $unsigned;
            let mut b = [0u8; MAX_BYTES];
            let mut i = 0usize;
            loop {
                b[i] = (val & 0b0111_1111) as u8;
                val >>= SHIFT;
                if val == 0 {
                    i = i + 1;
                    break;
                } else {
                    b[i] |= 0b1000_0000;
                    i = i + 1;
                }
            }
            self.raw_buffer(&b[0..i])?;
            Ok(self)
        }
    };
}

macro_rules! build_insert_varint {
    ($name:ident, $type:ty, $unsigned:ty) => {
        #[inline]
        #[allow(dead_code)]
        pub(crate) fn $name(
            &mut self,
            insertion: BinaryWriterInsertion,
            val: $type,
        ) -> Result<&mut Self, Error> {
            const SIZE: usize = std::mem::size_of::<$type>();
            const SHIFT: usize = 7;
            const BITSIZE: usize = SIZE * 8;
            const MAX_BYTES: usize = (BITSIZE + SHIFT - 1) / SHIFT;

            let mut val = val as $unsigned;
            let mut b = [0u8; MAX_BYTES];
            let mut i = 0usize;
            loop {
                b[i] = (val & 0b0111_1111) as u8;
                val >>= SHIFT;
                if val == 0 {
                    i = i + 1;
                    break;
                } else {
                    b[i] |= 0b1000_0000;
                    i = i + 1;
                }
            }
            self.insert_raw_buffer(insertion, &b[0..i])?;
            Ok(self)
        }
    };
}

macro_rules! build_write_fixnum {
    ($name:ident, $type:ty) => {
        #[inline]
        #[allow(dead_code)]
        pub(crate) fn $name(&mut self, val: $type) -> Result<&mut Self, Error> {
            self.raw_buffer(&val.to_be_bytes() as &[u8])?;
            Ok(self)
        }
    };
}

impl<W: Write + Unpin> BinaryWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            order: Vec::new(),
            buffer: Vec::new(),
            writer,
            cipher: None,
        }
    }

    #[inline]
    pub fn encrypt(&mut self, crypt: AesCfb8) -> &mut Self {
        self.cipher = Some(crypt);
        self
    }

    #[inline]
    pub(crate) fn raw_buffer(&mut self, data: &[u8]) -> Result<&mut Self, Error> {
        if data.len() == 0 {
            return Ok(self);
        }

        let start = self.buffer.len();
        self.buffer.extend_from_slice(data);
        let end = self.buffer.len();

        if let Some(order) = self.order.last_mut() {
            if let Some(order) = order {
                if order.end == start {
                    *order = order.start..end;
                    return Ok(self);
                }
            }
        }

        self.order.push(Some(start..end));

        Ok(self)
    }

    #[inline]
    pub(crate) fn insert_raw_buffer(
        &mut self,
        insertion: BinaryWriterInsertion,
        data: &[u8],
    ) -> Result<&mut Self, Error> {
        if data.len() == 0 {
            self.order[insertion.index] = Some(0..0);
            return Ok(self);
        }

        let start = self.buffer.len();
        self.buffer.extend_from_slice(data);
        let end = self.buffer.len();

        self.order[insertion.index] = Some(start..end);

        Ok(self)
    }

    #[inline]
    pub(crate) fn create_insertion(&mut self) -> BinaryWriterInsertion {
        let index = self.order.len();
        let start = self.buffer.len();
        self.order.push(None);
        BinaryWriterInsertion { start, index }
    }

    #[inline]
    pub(crate) fn bytes_after_insertion(&mut self, insertion: &BinaryWriterInsertion) -> usize {
        let current = self.buffer.len();
        current - insertion.start
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<(), Error> {
        if let Some(cipher) = self.cipher.as_mut() {
            for order in &self.order {
                if let Some(range) = order {
                    cipher.encrypt(&mut self.buffer[range.clone()]);
                } else {
                    return Err(ErrorKind::PendingInsertion.into());
                }
            }
        }

        for order in &self.order {
            if let Some(range) = order {
                match self.writer.write_all(&mut self.buffer[range.clone()]).await {
                    Ok(_) => (),
                    Err(e) => return Err(Box::new(e).into()),
                }
            } else {
                return Err(ErrorKind::PendingInsertion.into());
            }
        }

        self.buffer.clear();
        self.order.clear();

        Ok(())
    }

    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }

    #[inline]
    pub(crate) fn fix_bool(&mut self, val: bool) -> Result<&mut Self, Error> {
        self.fix_u8(if val { 1 } else { 0 })
    }

    build_write_fixnum!(fix_i8, i8);
    build_write_fixnum!(fix_i16, i16);
    build_write_fixnum!(fix_i32, i32);
    build_write_fixnum!(fix_i64, i64);

    build_write_fixnum!(fix_u8, u8);
    build_write_fixnum!(fix_u16, u16);
    build_write_fixnum!(fix_u32, u32);
    build_write_fixnum!(fix_u64, u64);

    build_write_varint!(var_i16, i16, u16);
    build_write_varint!(var_i32, i32, u32);
    build_write_varint!(var_i64, i64, u64);

    build_write_varint!(var_u16, u16, u16);
    build_write_varint!(var_u32, u32, u32);
    build_write_varint!(var_u64, u64, u64);

    build_write_fixnum!(fix_f32, f32);
    build_write_fixnum!(fix_f64, f64);

    build_insert_varint!(insert_var_i32, i32, u32);
}

pub(crate) struct BinaryWriterInsertion {
    start: usize,
    index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use cfb8::stream_cipher::NewStreamCipher;

    #[test]
    pub fn test_binary_writer_insertions() -> Result<(), Error> {
        let mut writer = make_writer();

        let pre = writer.create_insertion();
        writer.raw_buffer(b"1234" as &[u8])?;

        assert_eq!(4usize, writer.bytes_after_insertion(&pre));
        writer.insert_raw_buffer(pre, b"4567" as &[u8])?;

        let buf = make_buffer(writer);
        assert_eq!(buf, b"45671234");

        Ok(())
    }

    #[test]
    pub fn test_binary_writer_encryption() -> Result<(), Error> {
        let mut writer = make_writer();

        writer.encrypt(
            crate::AesCfb8::new_var(b"0234567890123456" as &[u8], b"0234567890123456" as &[u8])
                .unwrap(),
        );
        block_on(writer.fix_u8(1)?.fix_u8(10)?.flush()).unwrap();
        block_on(writer.fix_u8(1)?.fix_u8(10)?.flush()).unwrap();
        let buf = make_buffer(writer);
        assert_eq!(buf, b"\x2f\x57\xb5\x42");

        Ok(())
    }

    macro_rules! raw_write_tests {
        ($($name:ident: $writer:ident => $expr:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let mut $writer = make_writer();
                    $expr;
                    let buf = make_buffer($writer);
                    assert_eq!(buf, $expected);
                    Ok(())
                }
            )*
        }
    }

    raw_write_tests!(
        test_fix_bool_true: w => w.fix_bool(true)?, b"\x01",
        test_fix_bool_false: w => w.fix_bool(false)?, b"\x00",

        test_binary_writer_fix_u8: w => w.fix_u8(0x15)?, b"\x15",
        test_binary_writer_fix_u16: w => w.fix_u16(0x1526)?, b"\x15\x26",
        test_binary_writer_fix_u32: w => w.fix_u32(0x1526_3749)?, b"\x15\x26\x37\x49",
        test_binary_writer_fix_u64: w => w.fix_u64(0x1526_3749_5015_2637)?, b"\x15\x26\x37\x49\x50\x15\x26\x37",

        test_binary_writer_fix_i8: w => w.fix_i8(-0x15)?, b"\xeb",
        test_binary_writer_fix_i16: w => w.fix_i16(-0x1526)?, b"\xea\xda",
        test_binary_writer_fix_i32: w => w.fix_i32(-0x1526_3749)?, b"\xea\xd9\xc8\xb7",
        test_binary_writer_fix_i64: w => w.fix_i64(-0x1526_3749_5015_2637)?, b"\xea\xd9\xc8\xb6\xaf\xea\xd9\xc9",

        test_binary_writer_fix_f32: w => w.fix_f32(std::f32::consts::E)?, b"\x40\x2d\xf8\x54",
        test_binary_writer_fix_f64: w => w.fix_f64(std::f64::consts::E)?, b"\x40\x05\xbf\x0a\x8b\x14\x57\x69",

        // Test based on: https://wiki.vg/Protocol#VarInt_and_VarLong
        test_binary_writer_var_i16_p_0000: w => w.var_i16(0x0000)?, b"\x00",
        test_binary_writer_var_i16_p_0001: w => w.var_i16(0x0001)?, b"\x01",
        test_binary_writer_var_i16_p_0002: w => w.var_i16(0x0002)?, b"\x02",
        test_binary_writer_var_i16_p_007f: w => w.var_i16(0x007f)?, b"\x7f",
        test_binary_writer_var_i16_p_00ff: w => w.var_i16(0x00ff)?, b"\xff\x01",
        test_binary_writer_var_i16_p_7fff: w => w.var_i16(0x7fff)?, b"\xff\xff\x01",
        test_binary_writer_var_i16_m_0001: w => w.var_i16(-0x0001)?, b"\xff\xff\x03",
        test_binary_writer_var_i16_m_8000: w => w.var_i16(-0x8000)?, b"\x80\x80\x02",

        test_binary_writer_var_i32_p_0000_0000: w => w.var_i32(0x0000_0000)?, b"\x00",
        test_binary_writer_var_i32_p_0000_0001: w => w.var_i32(0x0000_0001)?, b"\x01",
        test_binary_writer_var_i32_p_0000_0002: w => w.var_i32(0x0000_0002)?, b"\x02",
        test_binary_writer_var_i32_p_0000_007f: w => w.var_i32(0x0000_007f)?, b"\x7f",
        test_binary_writer_var_i32_p_0000_00ff: w => w.var_i32(0x0000_00ff)?, b"\xff\x01",
        test_binary_writer_var_i32_p_7fff_ffff: w => w.var_i32(0x7fff_ffff)?, b"\xff\xff\xff\xff\x07",
        test_binary_writer_var_i32_m_0000_0001: w => w.var_i32(-0x0000_0001)?, b"\xff\xff\xff\xff\x0f",
        test_binary_writer_var_i32_m_8000_0000: w => w.var_i32(-0x8000_0000)?, b"\x80\x80\x80\x80\x08",

        test_binary_writer_var_i64_p_0000_0000_0000_0000: w => w.var_i64(0x0000_0000_0000_0000)?, b"\x00",
        test_binary_writer_var_i64_p_0000_0000_0000_0001: w => w.var_i64(0x0000_0000_0000_0001)?, b"\x01",
        test_binary_writer_var_i64_p_0000_0000_0000_0002: w => w.var_i64(0x0000_0000_0000_0002)?, b"\x02",
        test_binary_writer_var_i64_p_0000_0000_0000_007f: w => w.var_i64(0x0000_0000_0000_007f)?, b"\x7f",
        test_binary_writer_var_i64_p_0000_0000_0000_00ff: w => w.var_i64(0x0000_0000_0000_00ff)?, b"\xff\x01",
        test_binary_writer_var_i64_p_7fff_ffff_ffff_ffff: w => w.var_i64(0x7fff_ffff_ffff_ffff)?, b"\xff\xff\xff\xff\xff\xff\xff\xff\x7f",
        test_binary_writer_var_i64_m_0000_0000_0000_0001: w => w.var_i64(-0x0000_0000_0000_0001)?, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01",
        test_binary_writer_var_i64_m_8000_0000_0000_0000: w => w.var_i64(-0x8000_0000_0000_0000)?, b"\x80\x80\x80\x80\x80\x80\x80\x80\x80\x01",

        test_binary_writer_var_u16_0000: w => w.var_u16(0x0000)?, b"\x00",
        test_binary_writer_var_u16_0001: w => w.var_u16(0x0001)?, b"\x01",
        test_binary_writer_var_u16_0002: w => w.var_u16(0x0002)?, b"\x02",
        test_binary_writer_var_u16_007f: w => w.var_u16(0x007f)?, b"\x7f",
        test_binary_writer_var_u16_00ff: w => w.var_u16(0x00ff)?, b"\xff\x01",
        test_binary_writer_var_u16_7fff: w => w.var_u16(0x7fff)?, b"\xff\xff\x01",
        test_binary_writer_var_u16_ffff: w => w.var_u16(0xffff)?, b"\xff\xff\x03",

        test_binary_writer_var_u32_0000_0000: w => w.var_u32(0x0000_0000)?, b"\x00",
        test_binary_writer_var_u32_0000_0001: w => w.var_u32(0x0000_0001)?, b"\x01",
        test_binary_writer_var_u32_0000_0002: w => w.var_u32(0x0000_0002)?, b"\x02",
        test_binary_writer_var_u32_0000_007f: w => w.var_u32(0x0000_007f)?, b"\x7f",
        test_binary_writer_var_u32_0000_00ff: w => w.var_u32(0x0000_00ff)?, b"\xff\x01",
        test_binary_writer_var_u32_7fff_ffff: w => w.var_u32(0x7fff_ffff)?, b"\xff\xff\xff\xff\x07",
        test_binary_writer_var_u32_ffff_ffff: w => w.var_u32(0xffff_ffff)?, b"\xff\xff\xff\xff\x0f",

        test_binary_writer_var_u64_0000_0000_0000_0000: w => w.var_u64(0x0000_0000_0000_0000)?, b"\x00",
        test_binary_writer_var_u64_0000_0000_0000_0001: w => w.var_u64(0x0000_0000_0000_0001)?, b"\x01",
        test_binary_writer_var_u64_0000_0000_0000_0002: w => w.var_u64(0x0000_0000_0000_0002)?, b"\x02",
        test_binary_writer_var_u64_0000_0000_0000_007f: w => w.var_u64(0x0000_0000_0000_007f)?, b"\x7f",
        test_binary_writer_var_u64_0000_0000_0000_00ff: w => w.var_u64(0x0000_0000_0000_00ff)?, b"\xff\x01",
        test_binary_writer_var_u64_7fff_ffff_ffff_ffff: w => w.var_u64(0x7fff_ffff_ffff_ffff)?, b"\xff\xff\xff\xff\xff\xff\xff\xff\x7f",
        test_binary_writer_var_u64_ffff_ffff_ffff_ffff: w => w.var_u64(0xffff_ffff_ffff_ffff)?, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\x01"
    );
}
