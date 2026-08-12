use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Posts {
    pub posts: Vec<Post>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: u64,
    pub file: File,
    pub tags: Tags,
    pub sample: Sample,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct File {
    pub ext: String,
    pub url: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Tags {
    pub artist: Vec<String>,
    #[serde(default)]
    pub general: Vec<String>,
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Sample {
    pub has: bool,
    pub url: Option<String>,
    pub alternates: Alternates,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Alternates {
    #[serde(rename = "480p")]
    pub lower_quality: Option<LowerQuality>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LowerQuality {
    #[serde(rename = "type")]
    pub media_type: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolData {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub post_ids: Vec<u64>,
    pub post_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(artists: Vec<&str>) -> Tags {
        Tags {
            artist: artists.into_iter().map(String::from).collect(),
            general: vec![],
        }
    }

    #[test]
    fn parse_artists_none() {
        assert_eq!(tags(vec![]).parse_artists(), "unknown-artist");
    }

    #[test]
    fn parse_artists_one() {
        assert_eq!(tags(vec!["someartist"]).parse_artists(), "someartist");
    }

    #[test]
    fn parse_artists_many() {
        assert_eq!(
            tags(vec!["artist_a", "artist_b", "artist_c"]).parse_artists(),
            "artist_a, artist_b, artist_c"
        );
    }
}
