//! Message definitions

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Message {
    pub id: u8,
    pub data: [u8; 32],
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Data {
    Temperature(f32),
    RSAPublicKey(RSAPublicKey),
    Status,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RSAPublicKey {
    n: u64,
    e: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AESKey {
    key: [u8; 16],
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Status {
    Encrypted,
    Ok,
    Error,
}
