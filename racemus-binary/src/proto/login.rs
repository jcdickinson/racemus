use crate::{writer::StructuredWriter, BinaryWriter};
use async_std::io::Write;

pub enum LoginResponse<'a> {
    EncryptionRequest {
        public_key: &'a [u8],
        verify_token: &'a [u8],
    },
    Success {
        uuid: &'a str,
        player_name: &'a str,
    },
    Disconnect {
        reason: &'a str,
    },
}

impl<'a, W: Write + Unpin> StructuredWriter<W, LoginResponse<'a>> for BinaryWriter<W> {
    fn structure(&mut self, val: &LoginResponse<'a>) -> Result<&mut Self, crate::Error> {
        let insertion = self.create_insertion();
        match val {
            LoginResponse::EncryptionRequest {
                public_key,
                verify_token,
            } => self
                .var_i32(0x01)?
                .var_i32(0)?
                .arr_u8(public_key)?
                .arr_u8(verify_token)?,
            LoginResponse::Success { uuid, player_name } => {
                self.var_i32(0x02)?.arr_char(uuid)?.arr_char(player_name)?
            }
            LoginResponse::Disconnect { reason } => self.var_i32(0x00)?.arr_char(reason)?,
        }
        .insert_len_var_i32(insertion)
    }
}

#[cfg(test)]
mod tests {
    use super::{LoginResponse::*, *};
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
        test_write_login_encryption_request: w => w.structure(&EncryptionRequest{
            public_key: b"1234",
            verify_token: b"5678",
        })?, b"\x0c\x01\x00\x041234\x045678",
        test_write_login_login_success: w => w.structure(&Success{
            uuid: "1234",
            player_name: "5678"
        })?, b"\x0b\x02\x041234\x045678",
        test_write_login_disconnect: w => w.structure(&Disconnect{
            reason: "bad player"
        })?, b"\x0c\x00\x0abad player"
    );
}
