use crate::protocol::protocol_error::ProtocolErrorKind;
use nom::bytes::streaming::take;
use nom::Needed::Unknown;

macro_rules! build_varint {
    ($name:ident, $type:ty) => {
        pub fn $name<'a>(
            i: &'a [u8],
        ) -> nom::IResult<&'a [u8], $type, ProtocolErrorKind<&'a [u8]>> {
            const SIZE: usize = std::mem::size_of::<$type>() * 8;
            let mut res: usize = 0;
            let mut shift: usize = 0;
            let mut remainder = i;
            loop {
                let byte = match take::<usize, &[u8], ProtocolErrorKind<&[u8]>>(1)(remainder) {
                    Ok((rest, bytes)) => {
                        remainder = rest;
                        bytes[0]
                    }
                    Err(nom::Err::Incomplete(_)) => return Err(nom::Err::Incomplete(Unknown)),
                    Err(e) => return Err(e),
                };
                res |= ((byte as usize) & 0b0111_1111usize) << shift;
                if (byte & 0b1000_0000) == 0 {
                    return Ok((remainder, res as $type));
                }
                shift += 7;
                if shift > SIZE {
                    return std::result::Result::Err(nom::Err::Error(
                        ProtocolErrorKind::VarIntTooLarge(remainder),
                    ));
                }
            }
        }
    };
}

build_varint!(take_var_i32, i32);

#[macro_export]
macro_rules! build_utf8 {
    ($name:ident, $max:literal) => {
        pub fn $name<'a>(
            i: &'a [u8],
        ) -> nom::IResult<&'a [u8], &'a str, ProtocolErrorKind<&'a [u8]>> {
            let (i, sz) = take_var_i32(i)?;
            if (sz < 0 || ($max != 0 && sz > $max)) {
                return Err(nom::Err::Error(ProtocolErrorKind::StringTooLarge(i)));
            }
            let (i, bytes) = nom::bytes::streaming::take::<usize, &[u8], ProtocolErrorKind<&[u8]>>(
                sz as usize,
            )(i)?;
            match std::str::from_utf8(bytes) {
                Ok(st) => return Ok((i, st)),
                Err(e) => return Err(nom::Err::Error(ProtocolErrorKind::StringInvalid(i, e))),
            }
        }
    };
}

build_utf8!(take_utf8, 0);

#[macro_export]
macro_rules! build_buffer {
    ($name:ident, $max:literal) => {
        pub fn $name<'a>(
            i: &'a [u8],
        ) -> nom::IResult<&'a [u8], &'a [u8], ProtocolErrorKind<&'a [u8]>> {
            let (i, sz) = take_var_i32(i)?;
            if (sz < 0 || ($max != 0 && sz > $max)) {
                return Err(nom::Err::Error(ProtocolErrorKind::StringTooLarge(i)));
            }
            nom::bytes::streaming::take::<usize, &[u8], ProtocolErrorKind<&[u8]>>(sz as usize)(i)
        }
    };
}

build_buffer!(take_buffer, 0);

macro_rules! build_fixint {
    ($name:ident, $type:ty) => {
        pub fn $name<'a>(
            i: &'a [u8],
        ) -> nom::IResult<&'a [u8], $type, ProtocolErrorKind<&'a [u8]>> {
            const SIZE: usize = std::mem::size_of::<$type>();
            let (i, b) = take::<usize, &[u8], ProtocolErrorKind<&[u8]>>(SIZE)(i)?;
            let mut result = 0usize;
            for i in 0..SIZE {
                result = result << 8;
                result |= b[i] as usize;
            }
            Ok((i, result as $type))
        }
    };
}

build_fixint!(take_fix_u16, u16);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! parse_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr),*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(
                        $take_fn($input),
                        $expected
                    );
                }
            )*
        }
    }

    macro_rules! ok_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr, $remainder:expr),*) => {
            parse_tests! {
                $(
                    $name, $take_fn: $input, Ok(($remainder as &[u8], $expected))
                ),*
            }
        }
    }

    macro_rules! err_tests {
        ($($name:ident, $take_fn:ident: $input:expr, $expected:expr),*) => {
            parse_tests! {
                $(
                    $name, $take_fn: $input, Err(nom::Err::Error($expected))
                ),*
            }
        }
    }

    // Test vector from: https://wiki.vg/Protocol#VarInt_and_VarLong
    ok_tests! {
        take_var_i32_0, take_var_i32: b"\x00\x01\x02", 0x00, b"\x01\x02",
        take_var_i32_1, take_var_i32: b"\x01\x01\x02", 0x01, b"\x01\x02",
        take_var_i32_2, take_var_i32: b"\x02\x01\x02", 0x02, b"\x01\x02",
        take_var_i32_3, take_var_i32: b"\x7f\x01\x02", 0x7f, b"\x01\x02",
        take_var_i32_4, take_var_i32: b"\xff\x01\x02\x03", 0xff, b"\x02\x03",
        take_var_i32_5, take_var_i32: b"\xff\xff\xff\xff\x07\x02\x03", 0x7fff_ffff, b"\x02\x03",
        take_var_i32_6, take_var_i32: b"\xff\xff\xff\xff\x0f\x02\x03", -0x01, b"\x02\x03",
        take_var_i32_7, take_var_i32: b"\x80\x80\x80\x80\x08\x02\x03", -0x8000_0000, b"\x02\x03"
    }

    ok_tests! {
        take_utf8_0, take_utf8: b"\x1bFoo \xC2\xA9 bar \xF0\x9D\x8C\x86 baz \xE2\x98\x83 quxremainder", "Foo © bar 𝌆 baz ☃ qux", b"remainder"
    }

    build_utf8!(take_utf8_tiny, 3);

    err_tests! {
        take_utf8_tiny_0, take_utf8_tiny: b"\x1btest", ProtocolErrorKind::StringTooLarge(b"test" as &[u8])
    }

    ok_tests! {
        take_fix_u16_0, take_fix_u16: b"\x10\x20\x30\x40", 0x1020u16, b"\x30\x40"
    }

    ok_tests! {
        take_buffer_0, take_buffer: b"\x0a0123456789remainder", b"0123456789" as &[u8], b"remainder"
    }

    build_buffer!(take_buffer_tiny, 3);

    err_tests! {
        take_buffer_tiny_0, take_buffer_tiny: b"\x0atest", ProtocolErrorKind::StringTooLarge(b"test" as &[u8])
    }
}
