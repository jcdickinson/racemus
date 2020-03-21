pub mod insecure;

pub fn equals_constant_time(a: &[u8], b: &[u8]) -> bool {
    let mut eq = 0u8;

    for i in 0..a.len() {
        eq |= a[i] ^ b[i];
    }

    eq == 0
}
