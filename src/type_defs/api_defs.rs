use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Posts {
    pub posts: Vec<Post>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: u64,
    pub file: File,
    pub tags: Tags,
    pub sample: Sample,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    pub ext: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tags {
    pub artist: Vec<String>,
}

impl Tags {
    pub fn parse_artists(&self) -> String {
        match self.artist.len().cmp(&1) {
            Ordering::Greater => {
                let mut artists: String = String::new();
                for artist in self.artist.iter() {
                    artists = artists + artist + ", "
                }
                artists[..artists.len() - 2].to_string()
            }
            Ordering::Equal => self.artist[0].to_string(),
            Ordering::Less => "unknown-artist".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sample {
    pub has: bool,
    pub url: Option<String>,
    pub alternates: Alternates,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alternates {
    #[serde(rename = "480p")]
    pub lower_quality: Option<LowerQuality>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LowerQuality {
    #[serde(rename = "type")]
    pub media_type: String,
    pub urls: Vec<String>,
}
