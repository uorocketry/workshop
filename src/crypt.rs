//! RSA no std implementation
//! INSECURE: This is a toy implementation and should not be used in production code.

pub struct PublicKey {
    n: u64,
    e: u64,
}

pub struct PrivateKey {
    n: u64,
    d: u64,
}

impl PublicKey {
    pub fn new(n: u64, e: u64) -> PublicKey {
        PublicKey { n, e }
    }
}

impl PrivateKey {
    pub fn new(n: u64, d: u64) -> PrivateKey {
        PrivateKey { n, d }
    }
}

pub fn encrypt(pub_key: &PublicKey, m: u64) -> u64 {
    mod_exp(m, pub_key.e, pub_key.n)
}

pub fn decrypt(priv_key: &PrivateKey, c: u64) -> u64 {
    mod_exp(c, priv_key.d, priv_key.n)
}

fn mod_exp(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1;
    base = base % modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        exp = exp >> 1;
        base = (base * base) % modulus;
    }
    result
}