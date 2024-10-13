//! RSA no std implementation
//! INSECURE: This is a toy implementation and should not be used in production code.

use heapless::Vec;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RSAPublicKey {
    n: u64,
    e: u64,
}

#[derive(Clone)]
pub struct RSAPrivateKey {
    n: u64,
    d: u64,
}

impl RSAPublicKey {
    pub fn new(n: u64, e: u64) -> RSAPublicKey {
        RSAPublicKey { n, e }
    }
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        for i in 0..8 {
            bytes[i] = (self.n >> (i * 8)) as u8;
            bytes[i + 8] = (self.e >> (i * 8)) as u8;
        }
        bytes
    }
    pub fn from_bytes(bytes: &[u8; 16]) -> RSAPublicKey {
        let n = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let e = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        RSAPublicKey { n, e }
    }
}

impl RSAPrivateKey {
    pub fn new(n: u64, d: u64) -> RSAPrivateKey {
        RSAPrivateKey { n, d }
    }
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        for i in 0..8 {
            bytes[i] = (self.n >> (i * 8)) as u8;
            bytes[i + 8] = (self.d >> (i * 8)) as u8;
        }
        bytes
    }
    pub fn from_bytes(bytes: &[u8; 16]) -> RSAPrivateKey {
        let n = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let d = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        RSAPrivateKey { n, d }
    }
}

pub fn encrypt(pub_key: &RSAPublicKey, m: &[u8]) -> [u8; 8] {
    let m_int = bytes_to_u64(m);
    let encrypted_int = mod_exp(m_int, pub_key.e, pub_key.n);
    u64_to_bytes(encrypted_int)
}

pub fn decrypt(priv_key: &RSAPrivateKey, c: &[u8]) -> [u8; 8] {
    let c_int = bytes_to_u64(c);
    let decrypted_int = mod_exp(c_int, priv_key.d, priv_key.n);
    u64_to_bytes(decrypted_int)
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

fn bytes_to_u64(bytes: &[u8]) -> u64 {
    assert!(bytes.len() <= 8, "Array length exceeds 8 bytes");
    let mut result = 0u64;
    for &byte in bytes {
        result = (result << 8) | byte as u64;
    }
    result
}

fn u64_to_bytes(mut value: u64) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    for i in (0..8).rev() {
        bytes[i] = (value & 0xFF) as u8;
        value >>= 8;
    }
    bytes
}

// AES Encryption and Decryption
const AES_BLOCK_SIZE: usize = 8;

fn aes_encrypt_block(key: &[u8; 8], block: &mut [u8; 8]) {
    // Implement AES-128 encryption for a single block
    // This is a simplified example and not a full AES implementation
    for i in 0..AES_BLOCK_SIZE {
        block[i] ^= key[i];
    }
}

fn aes_decrypt_block(key: &[u8; 8], block: &mut [u8; 8]) {
    // Implement AES-128 decryption for a single block
    // This is a simplified example and not a full AES implementation
    for i in 0..AES_BLOCK_SIZE {
        block[i] ^= key[i];
    }
}

pub fn aes_encrypt(key: &[u8; 8], data: &[u8]) -> Vec<u8, 256> {
    let mut encrypted_data: Vec<u8, 256> = Vec::new();
    let mut iv = [0u8; AES_BLOCK_SIZE]; // Deterministic IV for simplicity

    for chunk in data.chunks(AES_BLOCK_SIZE) {
        let mut block = [0u8; AES_BLOCK_SIZE];
        block[..chunk.len()].copy_from_slice(chunk);
        for i in 0..AES_BLOCK_SIZE {
            block[i] ^= iv[i];
        }
        aes_encrypt_block(key, &mut block);
        iv = block;
        encrypted_data.extend_from_slice(&block).unwrap();
    }

    encrypted_data
}

pub fn aes_decrypt(key: &[u8; 8], encrypted_data: &[u8]) -> Vec<u8, 256> {
    let mut decrypted_data: Vec<u8, 256> = Vec::new();
    let mut iv = [0u8; AES_BLOCK_SIZE]; // Deterministic IV for simplicity

    for chunk in encrypted_data.chunks(AES_BLOCK_SIZE) {
        let mut block = [0u8; AES_BLOCK_SIZE];
        block.copy_from_slice(chunk);
        let mut decrypted_block = block;
        aes_decrypt_block(key, &mut decrypted_block);
        for i in 0..AES_BLOCK_SIZE {
            decrypted_block[i] ^= iv[i];
        }
        iv = block;
        decrypted_data.extend_from_slice(&decrypted_block).unwrap();
    }

    decrypted_data
}

// FNV-1a Hash Function
fn fnv1a_hash(data: &[u8]) -> [u8; 8] {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    let mut key = [0u8; 8];
    for i in 0..8 {
        key[i] = (hash >> (i * 8)) as u8;
    }
    key
}

// Generate AES Key
pub fn generate_aes_key(seed: &[u8]) -> [u8; 8] {
    fnv1a_hash(seed)
}
