mod error;
mod writer;
mod reader;
pub mod proto;

pub use error::*;
pub use writer::*;
pub use reader::*;

use aes::Aes128;
use cfb8::Cfb8;

pub const SERVER_VERSION: &str = "1.15.2";
pub const SERVER_VERSION_NUMBER: i32 = 578;

type AesCfb8 = Cfb8<Aes128>;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use async_std::io::Cursor;
    pub use async_std::task::block_on;

    pub fn make_writer() -> BinaryWriter<Cursor<Vec<u8>>> {
        let stream = Cursor::new(Vec::new());
        let writer = BinaryWriter::new(stream);
        writer
    }

    pub fn make_buffer(writer: BinaryWriter<Cursor<Vec<u8>>>) -> Vec<u8> {
        let mut writer = writer;
        block_on(writer.flush()).unwrap();
        writer.into_inner().into_inner()
    }
    
    pub fn make_reader(data: &[u8]) -> BinaryReader<Cursor<Vec<u8>>> {
        let stream = Cursor::new(data.to_vec());
        let reader = BinaryReader::new(stream);
        reader
    }
}