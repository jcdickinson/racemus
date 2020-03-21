use crate::{writer::StructuredWriter, BinaryWriter};
use async_std::io::Write;

pub enum OpenResponse {}

impl<W: Write + Unpin> StructuredWriter<W, OpenResponse> for BinaryWriter<W> {
    fn structure(&mut self, _: &OpenResponse) -> Result<&mut Self, crate::Error> {
        Ok(self)
    }
}
