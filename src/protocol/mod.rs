pub mod login;
pub mod open;
pub mod status;

use aes::Aes128;
use cfb8::stream_cipher::StreamCipher;
use cfb8::Cfb8;
use circular::Buffer;
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use async_std::io::{Read, Write};
use async_std::prelude::*;

pub const SERVER_VERSION: &str = "1.15.2";
pub const SERVER_VERSION_NUMBER: i32 = 578;

pub type AesCfb8 = Cfb8<Aes128>;

pub struct PacketWriter<W: Write + Unpin> {
    target: Vec<u8>,
    writer: W,
    cipher: Option<AesCfb8>
}

macro_rules! build_write_varint {
    ($name:ident, $type:ty) => {
        pub fn $name(&mut self, val: $type) -> &mut Self {
            let mut val = val as u64;
            loop {
                let b = (val & 0b0111_1111) as u8;
                val >>= 7;
    
                if val == 0 {
                    self.target.push(b);
                    break;
                } else {
                    self.target.push(b | 0b1000_0000);
                }
            }
            self
        }
    };
}
macro_rules! build_write_fixint {
    ($name:ident, $type:ty) => {
        pub fn $name(&mut self, val: $type) -> &mut Self {
            const SIZE: usize = std::mem::size_of::<$type>();
            let val = val as u64;
            for i in (0..SIZE).rev() {
                let v = (val >> (i * 8)) & 0b1111_1111;
                self.target.push(v as u8);
            }
            self
        }
    };
}

impl<W: Write + Unpin> PacketWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            target: Vec::new(),
            writer,
            cipher: None
        }
    }

    pub fn encrypt(&mut self, crypt: AesCfb8) -> &mut Self {
        self.cipher = Some(crypt);
        self
    }

    pub fn packet_id(&mut self, val: i32) -> &mut Self {
        self.var_i32(val);
        self
    }

    build_write_varint!(var_i32, i32);
    build_write_fixint!(fix_u8, u8);
    build_write_fixint!(fix_i32, i32);
    build_write_fixint!(fix_u64, u64);

    pub fn arr_u8(&mut self, val: &[u8]) -> &mut Self {
        self.var_i32(val.len() as i32);
        self.target.extend_from_slice(val);
        self
    }
    pub fn arr_char(&mut self, val: &str) -> &mut Self {
        self.arr_u8(val.as_bytes())
    }
    pub fn fix_bool(&mut self, val: bool) -> &mut Self {
        self.fix_u8(if val { 0x01 } else { 0x00 })
    }

    pub async fn flush(&mut self) -> Result<(), std::io::Error> {
        let index = self.target.len();
        self.var_i32(index as i32);

        if let Some(cipher) = self.cipher.as_mut() {
            cipher.encrypt(&mut self.target[index..]);
            cipher.encrypt(&mut self.target[..index]);
        };
        
        self.writer.write_all(&self.target[index..]).await?;
        self.writer.write_all(&self.target[..index]).await?;
        self.target.clear();

        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

pub async fn write_disconnect_login<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    reason: &str,
) -> Result<(), std::io::Error> {
    writer.packet_id(0x00).arr_char(reason);
    writer.flush().await
}

pub async fn write_disconnect_play<W: Write + Unpin>(
    writer: &mut PacketWriter<W>,
    reason: &str,
) -> Result<(), std::io::Error> {
    writer.packet_id(0x1b).arr_char(reason);
    writer.flush().await
}

pub struct PacketReader<R: Read + Unpin> {
    buffer: Buffer,
    current_len: usize,
    reader: R,
    cipher: Option<AesCfb8>
}
macro_rules! build_read_varint {
    ($name:ident, $type:ty) => {
        pub async fn $name(&mut self) -> Result<$type, Error> {
            const SIZE: usize = std::mem::size_of::<$type>() * 8;
            let mut res: u64 = 0;
            let mut shift: usize = 0;
            loop {
                let byte = self.fix_u8().await?;
                res |= ((byte as u64) & 0b0111_1111) << shift;
                if (byte & 0b1000_0000) == 0 {
                    return Ok(res as i32);
                }
                shift += 7;
                if shift > SIZE {
                    return Err(ErrorKind::InvalidData.into());
                }
            }
        }
    };
}
macro_rules! build_read_fixint {
    ($name:ident, $type:ty) => {
        pub async fn $name(&mut self) -> Result<$type, Error> {
            const SIZE: usize = std::mem::size_of::<$type>();
            if self.current_len < SIZE {
                return Err(ErrorKind::InvalidData.into());
            }
            if self.buffer.available_data() < SIZE {
                self.fill(SIZE).await?
            }
            let mut result = 0u64;
            for b in &self.buffer.data()[0..SIZE] {
                result <<= 8;
                result |= *b as u64;
            }
            self.current_len -= SIZE;
            self.buffer.consume(SIZE);
            Ok(result as $type)
        }
    };
}

impl<R: Read + Unpin> PacketReader<R> {
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
            current_len: 0,
            reader,
            cipher: None
        }
    }

    #[cfg(test)]
    pub fn with_length(reader: R, current_len: usize) -> Self {
        Self {
            buffer: Buffer::with_capacity(Self::BUFFER_INIT),
            current_len,
            reader,
            cipher: None
        }
    }

    pub fn decrypt(&mut self, cipher: AesCfb8) -> &mut Self {
        // We don't need to decrypt the data retroactively because the
        // encryption negotiation is lock-step.
        self.cipher = Some(cipher);
        self
    }

    async fn fill(&mut self, size: usize) -> Result<(), std::io::Error> {
        if size > self.buffer.available_space() {
            let size = size - self.buffer.available_data();
            
            // Size it to BUFFER_GROW increments
            let size = (size + Self::BUFFER_GROW - 1) / Self::BUFFER_GROW;
            let size = size * Self::BUFFER_GROW;

            self.buffer.grow(size);
        }
        while self.buffer.available_data() < size {
            let n = self.reader.read(&mut self.buffer.space()).await?;
            if n == 0 {
                return Err(ErrorKind::UnexpectedEof.into());
            }

            if let Some(cipher) = self.cipher.as_mut() {
                cipher.decrypt(&mut self.buffer.space()[0..n]);
            }
            self.buffer.fill(n);
        }
        Ok(())
    }

    build_read_fixint!(fix_u8, u8);
    build_read_fixint!(fix_u16, u16);
    build_read_fixint!(fix_u64, u64);
    build_read_varint!(var_i32, i32);
    
    async fn length_prefix(&mut self) -> Result<usize, Error> {
        let len = self.var_i32().await?;
        if len <= 0 {
            return Err(ErrorKind::InvalidData.into());
        }
        Ok(len as usize)
    }

    pub async fn packet_header(&mut self) -> Result<i32, Error> {
        // Consume remaining data.
        if self.current_len != 0 {
            let remove = std::cmp::min(self.current_len, self.buffer.available_data());
            self.current_len -= remove;
            self.buffer.consume(remove);
            while self.current_len != 0 {
                // We are not decrypting, so don't overfill.
                let remove = std::cmp::min(self.current_len, self.buffer.available_space());
                let remove = self
                    .reader
                    .read(&mut self.buffer.space()[0..remove])
                    .await?;
                if remove == 0 {
                    return Err(ErrorKind::UnexpectedEof.into());
                }
                self.current_len -= remove;
            }
        }

        // Provide space for the var_i32;
        self.current_len = 6;
        self.current_len = self.length_prefix().await?;
        self.var_i32().await
    }

    async fn raw_arr_u8(&mut self, max: Option<usize>) -> Result<&[u8], Error> {
        let len = self.length_prefix().await?;
        if len > self.current_len {
            return Err(ErrorKind::InvalidData.into());
        }
        if let Some(max) = max {
            if len > max {
                return Err(ErrorKind::InvalidData.into());
            }
        }
        if self.buffer.available_data() < len {
            self.fill(len).await?;
        }
        self.current_len -= len;
        Ok(&self.buffer.data()[0..len])
    }

    pub async fn arr_u8(&mut self, max: Option<usize>) -> Result<Vec<u8>, Error> {
        let slice = self.raw_arr_u8(max).await?;
        let vec = slice.to_vec();
        self.buffer.consume(vec.len());
        Ok(vec)
    }

    pub async fn arr_char(&mut self, max: Option<usize>) -> Result<String, Error> {
        let slice = self.raw_arr_u8(max).await?;
        let len = slice.len();
        match std::str::from_utf8(slice) {
            Ok(s) => {
                let s = s.to_string();
                self.buffer.consume(len);
                Ok(s)
            },
            Err(_) => Err(ErrorKind::InvalidData.into())
        }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task::block_on;
    use async_std::io::Cursor;
    use cfb8::stream_cipher::NewStreamCipher;

    macro_rules! sync {
        ($e:expr) => {
            block_on($e).unwrap()
        };
    }
    
    macro_rules! sync_err {
        ($e:expr) => {
            match block_on($e) {
                Ok(_) => None,
                Err(e) => Some(e.kind())
            }
        };
    }

    macro_rules! raw_write_tests {
        ($($name:ident: $writer:ident => $expr:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let target = Cursor::new(Vec::<u8>::new());
                    let mut $writer = PacketWriter::new(target);
                    $expr;
                    sync!($writer.flush());
                    assert_eq!($writer.into_inner().into_inner(), $expected as &[u8]);
                }
            )*
        }
    }
    macro_rules! write_tests {
        ($($name:ident: $fn:ident( $($param:expr),* ), $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    let target = Cursor::new(Vec::<u8>::new());
                    let mut writer = PacketWriter::new(target);
                    sync!(
                        $fn(
                            &mut writer,
                           $(
                            $param
                           ),*
                        )
                    );
                    assert_eq!(
                        writer.into_inner().into_inner(),
                        $expected as &[u8]
                    );
                }
            )*
        }
    }

    macro_rules! raw_read_tests {
        ($($name:ident, $input:literal: $reader:ident => { $($expr:expr, $expected:expr),* }),*) => {
            $(
                #[test]
                pub fn $name() {
                    let input = $input as &[u8];
                    let target = Cursor::new(input);
                    let mut $reader = PacketReader::with_length(target, input.len());
                    $({
                        assert_eq!(sync!($expr), $expected);
                    })*
                }
            )*
        }
    }

    macro_rules! raw_read_error_tests {
        ($($name:ident, $input:literal: $reader:ident => { $($expr:expr, $expected:expr),* }),*) => {
            $(
                #[test]
                pub fn $name() {
                    let input = $input as &[u8];
                    let target = Cursor::new(input);
                    let mut $reader = PacketReader::with_length(target, input.len());
                    $({
                        assert_eq!(sync_err!($expr), $expected);
                    })*
                }
            )*
        }
    }

    raw_write_tests! {
        packet_writer_packet_id: w => w.packet_id(50), b"\x01\x32",
        packet_writer_var_i32: w => w.packet_id(50).var_i32(453), b"\x03\x32\xc5\x03",
        packet_writer_fix_u8: w => w.packet_id(50).fix_u8(0x15), b"\x02\x32\x15",
        packet_writer_fix_i32: w => w.packet_id(50).fix_i32(0x1526_3748), b"\x05\x32\x15\x26\x37\x48",
        packet_writer_fix_u64: w => w.packet_id(50).fix_u64(0x1526_3748_5960_7182), b"\x09\x32\x15\x26\x37\x48\x59\x60\x71\x82",
        packet_writer_var_buffer: w => w
            .packet_id(50)
            .arr_u8(b"1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890" as &[u8]),
            b"\x85\x01\x32\x82\x011234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
        packet_writer_var_utf8: w => w.packet_id(50).arr_char("this is a string test 🎉✨"), b"\x1f\x32\x1dthis is a string test \xf0\x9f\x8e\x89\xe2\x9c\xa8",
        packet_writer_encrypt: w => w
            .packet_id(50).arr_char("test")
            .encrypt(AesCfb8::new_var(b"1234567890123456" as &[u8], b"1234567890123456" as &[u8]).unwrap()),
            b"\x73\xe5\x94\xa4\x6b\xd7\x91",
        packet_writer_encrypt_alternate: w => w
            .packet_id(50).arr_char("test")
            .encrypt(AesCfb8::new_var(b"0234567890123456" as &[u8], b"0234567890123456" as &[u8]).unwrap()),
            b"\x28\x11\xd4\x0a\xfe\x81\x42"
    }

    write_tests! {
        write_disconnect_login_test: write_disconnect_login("bad!"), b"\x06\x00\x04bad!" as &[u8],
        write_disconnect_play_test: write_disconnect_play("bad?"), b"\x06\x1b\x04bad?" as &[u8]
    }

    raw_read_tests! {
        // Test vector from: https://wiki.vg/Protocol#VarInt_and_VarLong
        read_var_i32_1, b"\x00\x01\x02\x7f": r => {
            r.var_i32(), 0x00,
            r.var_i32(), 0x01,
            r.var_i32(), 0x02,
            r.var_i32(), 0x7f
        },
        read_var_i32_2, b"\xff\x01": r => {
            r.var_i32(), 0xff
        },
        read_var_i32_3, b"\xff\xff\xff\xff\x07": r => {
            r.var_i32(), 0x7fff_ffff
        },
        read_var_i32_4, b"\xff\xff\xff\xff\x0f": r => {
            r.var_i32(), -0x01
        },
        read_var_i32_5, b"\x80\x80\x80\x80\x08": r => {
            r.var_i32(), -0x8000_0000
        },
        read_fix_u16, b"\x10\x20": r => {
            r.fix_u16(), 0x1020u16
        },
        read_fix_u64, b"\x10\x20\x30\x40\x50\x60\x70\x80": r => {
            r.fix_u64(), 0x1020_3040_5060_7080
        },
        read_arr_char, b"\x1bFoo \xC2\xA9 bar \xF0\x9D\x8C\x86 baz \xE2\x98\x83 qux": r => {
            r.arr_char(None), "Foo © bar 𝌆 baz ☃ qux"
        },
        read_arr_u8, b"\x0a0123456789": r => {
            r.arr_u8(None), b"0123456789" as &[u8]
        }
    }
    
    raw_read_error_tests! {
        read_arr_char_big, b"\x0a0123456789": r => {
            r.arr_char(Some(3)), Some(ErrorKind::InvalidData)
        },
        read_arr_u8_big, b"\x0a0123456789": r => {
            r.arr_u8(Some(3)), Some(ErrorKind::InvalidData)
        }
    }

    #[test]
    pub fn read_packet_header() {
        let input = "\x0a\x01234567890\x01\x02";
        let target = Cursor::new(input);
        let mut reader = PacketReader::new(target);
        assert_eq!(sync!(reader.packet_header()), 0x01);
        assert_eq!(sync!(reader.packet_header()), 0x02);
    }
    
    #[test]
    pub fn write_twice() {
        let mut writer = PacketWriter::new(Cursor::new(Vec::new()));

        writer.encrypt(AesCfb8::new_var(b"0234567890123456" as &[u8], b"0234567890123456" as &[u8]).unwrap());
        sync!(writer.packet_id(10).flush());
        sync!(writer.packet_id(10).flush());
        
        assert_eq!(writer.into_inner().into_inner(), b"\x2f\x57\xb5\x42");
    }
}
