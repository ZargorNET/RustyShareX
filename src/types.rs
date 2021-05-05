use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeaderDoc {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "delete_key")]
    pub delete_key: String,
    #[serde(rename = "content_type")]
    pub content_type: String,
    #[serde(rename = "file_extension")]
    pub file_extension: String,
    #[serde(rename = "content_length", with = "bson::compat::u2f")]
    pub content_length: u32,
    #[serde(rename = "uploaded_at", with = "bson::compat::u2f")]
    pub uploaded_at: u64,
    #[serde(rename = "total_chunks", with = "bson::compat::u2f")]
    pub total_chunks: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChunkDoc {
    #[serde(rename = "parent_id")]
    pub parent_id: String,
    pub index: i32,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}