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

    pub(crate) fn start_packet(&mut self) -> PacketInsertion {
        if self.compression_allowed() {
            PacketInsertion {
                uncompressed_length: Some(self.create_insertion()),
                raw_length: self.create_insertion(),
            }
        } else {
            PacketInsertion {
                uncompressed_length: None,
                raw_length: self.create_insertion(),
            }
        }
    }

    pub(crate) fn complete_packet(&mut self, packet: PacketInsertion) -> Result<&mut Self, Error> {
        let raw_length = packet.raw_length;
        if let Some(uncompressed_length) = packet.uncompressed_length {
            let original_len = self.bytes_after_insertion(&raw_length);
            if original_len > MAX_LEN {
                return Err(ErrorKind::LengthTooLarge.into());
            }

            match self.try_compress(&raw_length)? {
                Some(compressed_len) => {
                    let compressed_len =
                        compressed_len + self.insert_var_i32(raw_length, original_len as i32)?;
                    self.insert_var_i32(uncompressed_length, compressed_len as i32)?;
                }
                None => {
                    let original_len = original_len + self.insert_var_i32(raw_length, 0)?;
                    self.insert_var_i32(uncompressed_length, original_len as i32)?;
                }
            }
        } else {
            let len = self.bytes_after_insertion(&raw_length);
            if len > MAX_LEN {
                return Err(ErrorKind::LengthTooLarge.into());
            }
            self.insert_var_i32(raw_length, len as i32)?;
        }
        Ok(self)
    }
}

pub(crate) struct PacketInsertion {
    uncompressed_length: Option<BinaryWriterInsertion>,
    raw_length: BinaryWriterInsertion,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    pub fn binary_writer_complete_packet() -> Result<(), Error> {
        let mut writer = make_writer();

        let pre = writer.start_packet();
        writer.raw_buffer(b"1234" as &[u8])?;
        writer.complete_packet(pre)?;

        let buf = make_buffer(writer);
        assert_eq!(buf, b"\x041234");

        Ok(())
    }

    #[test]
    pub fn binary_writer_complete_packet_compressed() -> Result<(), Error> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let mut writer = make_writer();
        writer.allow_compression(0);

        let pre = writer.start_packet();
        let mut expected = "".to_string();
        for i in 1..1000 {
            expected.push_str(&i.to_string());
        }
        writer.raw_buffer(expected.as_bytes())?;
        writer.complete_packet(pre)?;

        let buf = make_buffer(writer);

        // This is not entirely deterministic and this may have to be updated if
        // the flate2 crate is updated.
        assert_eq!(buf[0..4], [0x98, 0x0a, 0xc9, 0x16]); // 1032, 2889

        let mut zlib = ZlibDecoder::new(&buf[4..]);
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

    raw_write_tests! {
        binary_writer_arr_char, "test-data/arr-char-1.in", w => w
            .arr_char("this is a string test 🎉✨")?
            .arr_char("this is a string test1 🎉✨")?;
        binary_writer_arr_u8, "test-data/arr-u8-1.in", w => w
            .arr_u8(b"12345")?
            .arr_u8(b"567890")?;
    }
}
