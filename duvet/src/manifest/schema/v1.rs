use duvet_core::{fs::PathId, manifests::Builder, Fs};
use serde::{de::Deserializer, Deserialize};

#[derive(Debug, Deserialize)]
pub struct Schema {
    pub compliance: Compliance,
}

impl Schema {
    pub fn load(&self, fs: &Fs, path_id: PathId, manifest: &mut Builder) {
        // TODO
    }
}

pub use compliance::Compliance;

pub mod compliance {
    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct Compliance {
        #[serde(rename = "source")]
        pub sources: Vec<Source>,

        #[serde(rename = "requirement")]
        pub requirements: Vec<Requirement>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Source {
        pub patterns: Vec<Pattern>,
        #[serde(rename = "comment-style")]
        pub comment_style: CommentStyle,
    }

    #[derive(Debug, Deserialize)]
    pub struct CommentStyle {
        pub meta: String,
        pub content: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Requirement {
        pub patterns: Vec<Pattern>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Spec {
        pub markdown: Vec<Markdown>,
        pub ietf: Vec<Ietf>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Markdown {
        pub patterns: Vec<Pattern>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Ietf {
        pub id: String,
        pub title: Option<String>,
        pub url: String,
        #[serde(default)]
        pub aliases: Vec<String>,
    }
}

#[derive(Debug)]
pub struct Pattern(globset::Glob);

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value = globset::Glob::new(&value).map_err(serde::de::Error::custom)?;
        Ok(Self(value))
    }
}
