pub mod login {
    pub const START: i32 = 0x00;
    pub const ENCRYPTION_RESPONSE: i32 = 0x01;

    pub const ENCRYPTION_REQUEST: i32 = 0x01;
    pub const SUCCESS: i32 = 0x02;
    pub const SET_COMPRESSION: i32 = 0x03;
    pub const DISCONNECT: i32 = 0x00;
}

pub mod open {
    pub const HANDSHAKE: i32 = 0x00;
}

pub mod play {
    pub const SERVER_DIFFICULTY: i32 = 0x0e;
    pub const PLUGIN: i32 = 0x19;
    pub const DISCONNECT: i32 = 0x1b;
    pub const JOIN_GAME: i32 = 0x26;
    pub const SET_POSITION_AND_LOOK: i32 = 0x36;
    pub const HELD_ITEM_CHANGE: i32 = 0x40;
}

pub mod status {
    pub const INFO_REQUEST: i32 = 0x00;
    pub const PING: i32 = 0x01;
    pub const INFO_RESPONSE: i32 = 0x00;
    pub const PONG: i32 = 0x01;
}
