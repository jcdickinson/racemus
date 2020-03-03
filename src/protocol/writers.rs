use std::marker::Unpin;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use aes::Aes128;
use cfb8::stream_cipher::StreamCipher;
use cfb8::Cfb8;

pub type AesCfb8 = Cfb8<Aes128>;

pub struct PacketWriter {
    target: Vec<Vec<u8>>,
    len: usize,
}

impl PacketWriter {
    pub fn new(id: i32) -> PacketWriter {
        let mut result = PacketWriter {
            target: Vec::new(),
            len: 0,
        };
        result.var_i32(id);
        result
    }
    pub fn var_i32(&mut self, val: i32) {
        let mut val = val as u32;
        let mut buf = Vec::with_capacity(3);
        loop {
            let b = (val & 0b0111_1111) as u8;
            val = val >> 7;

            if val == 0 {
                buf.push(b);
                self.len += buf.len();
                self.target.push(buf);
                break;
            } else {
                buf.push(b | 0b1000_0000);
            }
        }
    }

    pub fn var_buffer(&mut self, val: &[u8]) {
        let clone = Vec::from(val);
        self.var_i32(val.len() as i32);
        self.target.push(clone);
        self.len += val.len();
    }
    pub fn var_utf8(&mut self, val: &str) {
        self.var_buffer(val.as_bytes());
    }

    pub async fn flush<W: AsyncWrite + Unpin>(
        mut self,
        writer: &mut W,
        crypt: Option<&mut AesCfb8>,
    ) -> Result<(), std::io::Error> {
        self.var_i32(self.len as i32);
        let index = self.target.len() - 1;

        if let Some(crypt) = crypt {
            crypt.encrypt(&mut self.target[index]);
            for i in 0..index {
                crypt.encrypt(&mut self.target[i]);
            }
        };
        writer.write_all(&self.target[index]).await?;
        for i in 0..index {
            writer.write_all(&self.target[i]).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfb8::stream_cipher::NewStreamCipher;
    use futures::executor::block_on;
    use std::io::Cursor;

    #[test]
    pub fn packet_writer_new() {
        let mut target = Cursor::new(Vec::<u8>::new());
        block_on(PacketWriter::new(50).flush(&mut target, None)).unwrap();
        assert_eq!(target.into_inner(), b"\x01\x32");
    }

    #[test]
    pub fn packet_writer_var_i32() {
        let mut target = Cursor::new(Vec::<u8>::new());
        let mut writer = PacketWriter::new(50);
        writer.var_i32(453);
        block_on(writer.flush(&mut target, None)).unwrap();
        assert_eq!(target.into_inner(), b"\x03\x32\xc5\x03");
    }

    #[test]
    pub fn packet_writer_var_buffer() {
        let mut target = Cursor::new(Vec::<u8>::new());
        let mut writer = PacketWriter::new(50);
        writer.var_buffer(b"1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890" as &[u8]);
        block_on(writer.flush(&mut target, None)).unwrap();
        assert_eq!(target.into_inner(), b"\x85\x01\x32\x82\x011234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890" as &[u8]);
    }

    #[test]
    pub fn packet_writer_var_utf8() {
        let mut target = Cursor::new(Vec::<u8>::new());
        let mut writer = PacketWriter::new(50);
        writer.var_utf8("this is a string test 🎉✨");
        block_on(writer.flush(&mut target, None)).unwrap();
        assert_eq!(
            target.into_inner(),
            b"\x1f\x32\x1dthis is a string test \xf0\x9f\x8e\x89\xe2\x9c\xa8" as &[u8]
        );
    }

    #[test]
    pub fn packet_writer_encrypt() {
        let mut target = Cursor::new(Vec::<u8>::new());
        let mut writer = PacketWriter::new(50);
        writer.var_utf8("test");
        let mut aes =
            AesCfb8::new_var(b"1234567890123456" as &[u8], b"1234567890123456" as &[u8]).unwrap();
        block_on(writer.flush(&mut target, Some(&mut aes))).unwrap();
        assert_eq!(
            target.into_inner(),
            b"\x73\xe5\x94\xa4\x6b\xd7\x91" as &[u8]
        );
    }

    #[test]
    pub fn packet_writer_encrypt_alternate() {
        let mut target = Cursor::new(Vec::<u8>::new());
        let mut writer = PacketWriter::new(50);
        writer.var_utf8("test");
        let mut aes =
            AesCfb8::new_var(b"0234567890123456" as &[u8], b"0234567890123456" as &[u8]).unwrap();
        block_on(writer.flush(&mut target, Some(&mut aes))).unwrap();
        assert_eq!(
            target.into_inner(),
            b"\x28\x11\xd4\x0a\xfe\x81\x42" as &[u8]
        );
    }
}
