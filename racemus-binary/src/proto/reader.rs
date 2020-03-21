use crate::{BinaryReader, Error, ErrorKind};
use async_std::io::Read;
use std::marker::Unpin;

impl<R: Read + Unpin> BinaryReader<R> {
    #[inline]
    #[allow(dead_code)]
    async fn length_prefix(&mut self, max: Option<usize>) -> Result<usize, Error> {
        let len = self.var_i32().await?;
        if len <= 0 {
            return Err(ErrorKind::InvalidLengthPrefix.into());
        }

        let len = len as usize;
        if let Some(max) = max {
            if len > max {
                return Err(ErrorKind::InvalidLengthPrefix.into());
            }
        }

        if let Some(current_len) = self.current_len() {
            if len > current_len {
                return Err(ErrorKind::ReadPastPacket.into());
            }
        }

        Ok(len)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    pub fn test_binary_reader_length_prefix_happy_path() -> Result<(), Error> {
        let mut reader = make_reader(b"\x2f");

        assert_eq!(block_on(reader.length_prefix(None))?, 0x2f);

        Ok(())
    }
    
    #[test]
    pub fn test_binary_reader_length_prefix_field_long() -> Result<(), Error> {
        let mut reader = make_reader(b"\x7f");
        match block_on(reader.length_prefix(Some(5))) {
            Ok(r) => { assert_ne!(r, r) },
            Err(e) => match e.kind() {
                ErrorKind::InvalidLengthPrefix => {},
                _ => return Err(e)
            }
        }

        Ok(())
    }
    
    #[test]
    pub fn test_binary_reader_length_prefix_packet_long() -> Result<(), Error> {
        let mut reader = make_reader(b"\x7f");
        block_on(reader.with_size(Some(5))).unwrap();
        match block_on(reader.length_prefix(None)) {
            Ok(r) => { assert_ne!(r, r) },
            Err(e) => match e.kind() {
                ErrorKind::ReadPastPacket => {},
                _ => return Err(e)
            }
        }

        Ok(())
    }
}
