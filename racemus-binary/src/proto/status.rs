use crate::{writer::StructuredWriter, BinaryWriter};
use async_std::io::Write;

pub enum StatusResponse {
    Pong { timestamp: u64 },
}

impl<W: Write + Unpin> StructuredWriter<W, StatusResponse> for BinaryWriter<W> {
    fn structure(&mut self, val: &StatusResponse) -> Result<&mut Self, crate::Error> {
        let insertion = self.create_insertion();
        match val {
            StatusResponse::Pong { timestamp } => self.var_i32(0x01)?.fix_u64(*timestamp)?,
        }
        .insert_len_var_i32(insertion)
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusResponse::*, *};
    use crate::{tests::*, Error};

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
        test_write_status_pong: w => w.structure(&Pong{
            timestamp: 0x1526_3749_5015_2637
        })?, b"\x09\x01\x15\x26\x37\x49\x50\x15\x26\x37"
    );
}
