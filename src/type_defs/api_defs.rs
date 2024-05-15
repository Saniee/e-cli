//! Api definitions for the E api.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Posts {
    pub posts: Vec<Post>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: u64,
    pub file: File,
    pub tags: Tags,
    pub sample: Sample,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub ext: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub artist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sample {
    pub has: bool,
    pub url: Option<String>,
    pub alternates: Alternates,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Alternates {
    #[serde(rename = "480p")]
    pub lower_quality: Option<LowerQuality>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LowerQuality {
    #[serde(rename = "type")]
    pub media_type: String,
    pub urls: Vec<String>,
}
