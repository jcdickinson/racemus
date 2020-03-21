const CHUNK_SIZE: usize = 16;

struct LightLayer([u8; CHUNK_SIZE * CHUNK_SIZE]);

impl LightLayer {
    pub fn new() -> Self {
        Self([0u64; 16])
    }

    pub fn set(x: u8, z: u8, level: u8) {
        let level = level & 0b1111;
    }
}

pub struct LightData {

}