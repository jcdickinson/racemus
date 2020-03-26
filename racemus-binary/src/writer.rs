use crate::AesCfb8;
use crate::{Error, ErrorKind};
use async_std::io::{prelude::*, Write};
use cfb8::stream_cipher::StreamCipher;
use flate2::{write::ZlibEncoder, Compression};
use std::{marker::Unpin, ops::Range};

pub trait StructuredWriter<W: Write + Unpin, T> {
    fn structure(&mut self, val: &T) -> Result<&mut Self, Error>;
}

pub struct BinaryWriter<W: Write + Unpin> {
    order: Vec<Option<Range<usize>>>,
    buffer: Vec<u8>,
    writer: W,
    cipher: Option<AesCfb8>,
    compression_buffer: Option<Vec<u8>>,
    compression_threshold: Option<usize>,
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
        ) -> Result<usize, Error> {
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
            Ok(i)
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
            compression_buffer: None,
            compression_threshold: None,
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
                    Err(e) => return Err(e.into()),
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

    #[inline]
    pub fn allow_compression(&mut self, threshold: usize) {
        self.compression_threshold = Some(threshold);
    }

    #[inline]
    pub(crate) fn compression_allowed(&self) -> bool {
        self.compression_threshold.is_some()
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn try_compress(
        &mut self,
        after: &BinaryWriterInsertion,
    ) -> Result<Option<usize>, Error> {
        use std::io::Write;

        let threshold = match self.compression_threshold {
            Some(r) => r,
            None => return Ok(None),
        };

        // Only compress if it is over the threshold
        if self.bytes_after_insertion(&after) < threshold {
            return Ok(None);
        }

        let buffer = match std::mem::take(&mut self.compression_buffer) {
            Some(b) => b,
            None => Vec::new(),
        };

        // Data needs to follow
        if after.index == self.order.len() {
            return Err(ErrorKind::InvalidOperation.into());
        }

        // Collapse 1..2, 2..5, 5..10 into 1..10
        let mut data_range: Option<Range<usize>> = None;
        let order_range = (after.index + 1)..self.order.len();
        for i in order_range.clone() {
            let current = &self.order[i];
            match current {
                Some(current) => {
                    if let Some(previous) = data_range {
                        if current.start == previous.end {
                            data_range = Some(previous.start..current.end)
                        } else {
                            return Err(ErrorKind::InvalidOperation.into());
                        }
                    } else {
                        data_range = Some(current.clone());
                    }
                }
                // All subsequent length prefixes must be resolved
                None => return Err(ErrorKind::InvalidOperation.into()),
            }
        }

        let data_range = match data_range {
            Some(r) => r,
            None => return Err(ErrorKind::InvalidOperation.into()),
        };

        let mut zlib = ZlibEncoder::new(buffer, Compression::fast());
        zlib.write_all(&self.buffer[data_range.clone()])?;
        let mut buffer = zlib.finish()?;

        let result = if buffer.len() > data_range.len() {
            // Compression resulted in a larger packet, so just throw the result away
            buffer.truncate(0);
            Ok(None)
        } else {
            self.buffer.truncate(self.buffer.len() - data_range.len());
            self.buffer.append(&mut buffer);

            let data_range = data_range.start..self.buffer.len();
            self.order.truncate(self.order.len() - order_range.len());
            self.order.push(Some(data_range.clone()));
            Ok(Some(data_range.len()))
        };

        self.compression_buffer = Some(buffer);
        result
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
    pub fn binary_writer_insertions() -> Result<(), Error> {
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
    pub fn binary_writer_encryption() -> Result<(), Error> {
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

    #[test]
    pub fn binary_writer_compression_threshold() -> Result<(), Error> {
        let mut writer = make_writer();

        let pre = writer.create_insertion();
        let mut expected = String::new();
        for i in 1..1000 {
            expected.push_str(&i.to_string());
        }
        writer.raw_buffer(expected.as_bytes())?;

        writer.allow_compression(writer.buffer.len() + 1);
        match writer.try_compress(&pre)? {
            Some(_) => panic!("threshold not exceeded"),
            None => {}
        }

        // The location of the data here is based on how length prefixes work at the time of writing the test. If that
        // changes, then so must the test.
        let buf = std::mem::take(&mut writer.buffer);
        let order = writer.order[1].clone().unwrap();
        let mut reader = &buf[order];
        let mut actual = String::new();
        block_on(reader.read_to_string(&mut actual))?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    pub fn binary_writer_compression_skip() -> Result<(), Error> {
        let mut writer = make_writer();

        let pre = writer.create_insertion();
        let expected = "smol".to_string();
        writer.raw_buffer(expected.as_bytes())?;

        writer.allow_compression(0);
        match writer.try_compress(&pre)? {
            Some(_) => panic!("couldn't possibly be smaller"),
            None => {}
        }

        // The location of the data here is based on how length prefixes work at the time of writing the test. If that
        // changes, then so must the test.
        let buf = std::mem::take(&mut writer.buffer);
        let order = writer.order[1].clone().unwrap();
        let mut reader = &buf[order];
        let mut actual = String::new();
        block_on(reader.read_to_string(&mut actual))?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    pub fn binary_writer_compression() -> Result<(), Error> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let mut writer = make_writer();

        let pre = writer.create_insertion();
        let mut expected = String::new();

        for i in 1..1000 {
            expected.push_str(&i.to_string());
        }
        writer.raw_buffer(expected.as_bytes())?;

        let data_before = writer.bytes_after_insertion(&pre);
        writer.allow_compression(0);
        match writer.try_compress(&pre)? {
            Some(data_after) => assert!(data_before > data_after),
            _ => panic!("expected a smaller buffer"),
        }

        // The location of the data here is based on how insertion points work at the time of writing the test. If that
        // changes, then so must the test.
        let buf = std::mem::take(&mut writer.buffer);
        let order = writer.order[1].clone().unwrap();
        let mut zlib = ZlibDecoder::new(&buf[order]);
        let mut actual = String::new();
        zlib.read_to_string(&mut actual)?;

        assert_eq!(actual, expected);

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
        binary_writer_fix_bool_true, "test-data/fix-bool-1.in", w => w
            .fix_bool(false)?
            .fix_bool(true)?;

        binary_writer_fix_unsiged, "test-data/fix-unsigned-1.in", w => w
            .fix_u8(0x15)?
            .fix_u16(0x1526)?
            .fix_u32(0x1526_3749)?
            .fix_u64(0x1526_3749_5015_2637)?;
        binary_writer_fix_signed, "test-data/fix-signed-1.in", w => w
            .fix_i8(-0x15)?
            .fix_i16(-0x1526)?
            .fix_i32(-0x1526_3749)?
            .fix_i64(-0x1526_3749_5015_2637)?;
        binary_writer_fix_float, "test-data/fix-float-1.in", w => w
            .fix_f32(std::f32::consts::E)?
            .fix_f64(std::f64::consts::E)?;

        // Test vectors based on: https://wiki.vg/Protocol#VarInt_and_VarLong
        binary_writer_var_i16, "test-data/var-signed-16-1.in", w => w
            .var_i16(0x0000)?
            .var_i16(0x0001)?
            .var_i16(0x0002)?
            .var_i16(0x007f)?
            .var_i16(0x00ff)?
            .var_i16(0x7fff)?
            .var_i16(-0x0001)?
            .var_i16(-0x8000)?;
        binary_writer_var_i32, "test-data/var-signed-32-1.in", w => w
            .var_i32(0x0000_0000)?
            .var_i32(0x0000_0001)?
            .var_i32(0x0000_0002)?
            .var_i32(0x0000_007f)?
            .var_i32(0x0000_00ff)?
            .var_i32(0x7fff_ffff)?
            .var_i32(-0x0000_0001)?
            .var_i32(-0x8000_0000)?;
        binary_writer_var_i64, "test-data/var-signed-64-1.in", w => w
            .var_i64(0x0000_0000_0000_0000)?
            .var_i64(0x0000_0000_0000_0001)?
            .var_i64(0x0000_0000_0000_0002)?
            .var_i64(0x0000_0000_0000_007f)?
            .var_i64(0x0000_0000_0000_00ff)?
            .var_i64(0x7fff_ffff_ffff_ffff)?
            .var_i64(-0x0000_0000_0000_0001)?
            .var_i64(-0x8000_0000_0000_0000)?;

        binary_writer_var_u16, "test-data/var-unsigned-16-1.in", w => w
            .var_u16(0x0000)?
            .var_u16(0x0001)?
            .var_u16(0x0002)?
            .var_u16(0x007f)?
            .var_u16(0x00ff)?
            .var_u16(0x7fff)?
            .var_u16(0xffff)?;
        binary_writer_var_u32, "test-data/var-unsigned-32-1.in", w => w
            .var_u32(0x0000_0000)?
            .var_u32(0x0000_0001)?
            .var_u32(0x0000_0002)?
            .var_u32(0x0000_007f)?
            .var_u32(0x0000_00ff)?
            .var_u32(0x7fff_ffff)?
            .var_u32(0xffff_ffff)?;
        binary_writer_var_u64, "test-data/var-unsigned-64-1.in", w => w
            .var_u64(0x0000_0000_0000_0000)?
            .var_u64(0x0000_0000_0000_0001)?
            .var_u64(0x0000_0000_0000_0002)?
            .var_u64(0x0000_0000_0000_007f)?
            .var_u64(0x0000_0000_0000_00ff)?
            .var_u64(0x7fff_ffff_ffff_ffff)?
            .var_u64(0xffff_ffff_ffff_ffff)?;
    );
}
