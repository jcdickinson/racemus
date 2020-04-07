use log::trace;
use ring::digest;
use serde_derive::Deserialize;
use std::error::Error;

use async_std::io::prelude::*;
use async_std::net::TcpStream;
use http_types::{Method, Request, Url};

fn to_hex(v: u8) -> char {
    match v {
        0x0 => '0',
        0x1 => '1',
        0x2 => '2',
        0x3 => '3',
        0x4 => '4',
        0x5 => '5',
        0x6 => '6',
        0x7 => '7',
        0x8 => '8',
        0x9 => '9',
        0xa => 'a',
        0xb => 'b',
        0xc => 'c',
        0xd => 'd',
        0xe => 'e',
        _ => 'f',
    }
}

fn calculate_server_hash(server_id: &[u8], shared_secret: &[u8], public_key_der: &[u8]) -> String {
    let mut ctx = digest::Context::new(&digest::SHA1_FOR_LEGACY_USE_ONLY);
    ctx.update(server_id);
    ctx.update(shared_secret);
    ctx.update(public_key_der);
    let d = ctx.finish();
    let hash = d.as_ref();

    let negative = (hash[0] & 0x80) == 0x80;
    let copy = &mut [0u8; digest::SHA1_OUTPUT_LEN];

    let twos = if negative {
        let mut carry = true;
        for i in (0..hash.len()).rev() {
            copy[i] = !hash[i];
            if carry {
                carry = copy[i] == 0xff;
                copy[i] += 1;
            }
        }
        copy
    } else {
        hash
    };

    let result = &mut ['-'; 1 + digest::SHA1_OUTPUT_LEN * 2];

    let mut nonzero = false;
    let mut j = 1;
    for i in twos {
        let c = i >> 4;
        nonzero |= c != 0;
        if nonzero {
            result[j] = to_hex(c);
            j += 1;
        }

        let c = i & 0b1111;
        nonzero |= c != 0;
        if nonzero {
            result[j] = to_hex(c);
            j += 1;
        }
    }

    if negative {
        result[0..j].iter().collect()
    } else {
        result[1..j].iter().collect()
    }
}

const HAS_JOINED: &str = "https://sessionserver.mojang.com/session/minecraft/hasJoined";

#[derive(Deserialize)]
struct HasJoinedResponse {
    id: String,
    name: String,
}

impl From<HasJoinedResponse> for crate::api::PlayerInfo {
    fn from(value: HasJoinedResponse) -> Self {
        let mut uuid = value.id;
        uuid.insert(20, '-');
        uuid.insert(16, '-');
        uuid.insert(12, '-');
        uuid.insert(8, '-');

        let name = value.name;

        crate::api::PlayerInfo::new(name, uuid)
    }
}

pub async fn has_joined(
    player_name: &str,
    server_id: &[u8],
    shared_secret: &[u8],
    public_key_der: &[u8],
) -> Result<crate::api::PlayerInfo, Box<dyn Error + Send + Sync + 'static>> {
    let server_hash = calculate_server_hash(server_id, shared_secret, public_key_der);
    let mut url = Url::parse(HAS_JOINED).unwrap();

    let mut host = url.host_str().unwrap().to_string();
    host.push_str(":443");
    let stream = TcpStream::connect(&host).await?;

    url.query_pairs_mut()
        .append_pair("username", &player_name)
        .append_pair("serverId", &server_hash);
    trace!("sending login request: {}", url);

    let req = Request::new(Method::Get, url);
    let stream = async_native_tls::connect(req.url().host_str().unwrap(), stream).await?;
    let mut resp = async_h1::connect(stream, req).await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(Box::new(crate::api::ApiError::HttpStatus(status.into())));
    }

    let mut resp = resp.take_body().into_reader();
    let mut buffer = Vec::new();
    resp.read_to_end(&mut buffer).await?;
    let resp: HasJoinedResponse = serde_json::from_reader(std::io::Cursor::new(buffer))?;

    Ok(resp.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! hash_tests {
        ($($name:ident: $st:literal => $expected:literal),*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(calculate_server_hash(b"" as &[u8], b"" as &[u8], $st as &[u8]), $expected.to_string());
                }
            )*
        }
    }

    hash_tests! {
        hash_test_notch: b"Notch" => "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48",
        hash_test_jeb: b"jeb_" => "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1",
        hash_test_simon: b"simon" => "88e16a1019277b15d58faf0541e11910eb756f6"
    }
}
