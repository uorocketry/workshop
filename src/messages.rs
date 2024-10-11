//! Message definitions


#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Message {
    pub id: u8,
    pub data: Data,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Data {
    Temperature(f32),
    Status,  
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Status {
    Encrypted,
    Ok,
    Error,
}