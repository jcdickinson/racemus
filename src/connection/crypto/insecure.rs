use num::bigint::{BigUint, ToBigUint};
use num::{One, Zero};
use ring::io::der;

#[derive(Clone)]
pub struct InsecurePrivateKey {
    n: BigUint,
    d: BigUint,
    p: Vec<u8>,
}

impl std::fmt::Debug for InsecurePrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "InsecutePrivateKey")
    }
}

impl InsecurePrivateKey {
    pub fn from_der(input: &[u8], p: &[u8]) -> Result<InsecurePrivateKey, ()> {
        let (n, d) = untrusted::Input::from(input).read_all((), |input| {
            der::nested(input, der::Tag::Sequence, (), Self::from_der_reader)
        })?;
        Ok(InsecurePrivateKey { n, d, p: p.into() })
    }

    pub fn public_der(&self) -> &[u8] {
        &self.p
    }

    pub fn decrypt(&self, input: &[u8]) -> Vec<u8> {
        let c = BigUint::from_bytes_be(input);
        let v = Self::mod_exp(&c, &self.d, &self.n);
        v.to_bytes_be()
    }

    // Modular exponentiation by squaring
    fn mod_exp(base: &BigUint, exponent: &BigUint, modulus: &BigUint) -> BigUint {
        let mut result = One::one();
        let mut b = base.to_owned();
        let mut exp = exponent.to_owned();

        while exp > Zero::zero() {
            // Accumulate current base if current exponent bit is 1
            if (&exp & 1.to_biguint().unwrap()) == One::one() {
                result *= &b;
                result %= modulus;
            }
            // Get next base by squaring
            b = &b * &b;
            b = &b % modulus;

            // Get next bit of exponent
            exp >>= 1;
        }
        result
    }

    fn from_der_reader<'a>(input: &mut untrusted::Reader<'a>) -> Result<(BigUint, BigUint), ()> {
        let version =
            der::small_nonnegative_integer(input).map_err(|ring::error::Unspecified| ())?;
        if version != 0 {
            return Err(());
        }

        fn positive_integer<'a>(
            input: &mut untrusted::Reader<'a>,
        ) -> Result<ring::io::Positive<'a>, ()> {
            ring::io::der::positive_integer(input).map_err(|ring::error::Unspecified| ())
        }

        let n = positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?;
        let d = positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?.big_endian_without_leading_zero();
        positive_integer(input)?.big_endian_without_leading_zero();

        let n = BigUint::from_bytes_be(n);
        let d = BigUint::from_bytes_be(d);

        Ok((n, d))
    }
}
