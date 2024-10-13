//! Message definitions

use crate::crypt::RSAPublicKey;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Message {
    pub id: u8,
    pub data: Data,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Data {
    RSAPublicKey(RSAPublicKey),
    Status(Status),
    Command(Command),
    AESKey([u8; 8]),
    Temperature([u8; 32]),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Temperature {
    pub temp: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Status {
    UnkownPublicKey,
    // We cannot find the AES key in the EEPROM
    UnkownAESKey,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Command {
    DeleteAESKey,
}
