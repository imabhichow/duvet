use crate::Result;
use duvet_core::{fs::PathId, manifests::Builder, Fs};
use std::path::Path;

pub mod v1;

pub enum Schema {
    V1(v1::Schema),
}

impl Schema {
    pub fn parse(file: &Path, contents: &str) -> Result<Self> {
        match file.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => {
                // TODO add version entry
                let manifest = toml::from_str(contents)?;
                Ok(Self::V1(manifest))
            }
            ext => unimplemented!("{:?}", ext),
        }
    }

    pub fn load(&self, fs: &Fs, path_id: PathId, manifest: &mut Builder) {
        match self {
            Self::V1(schema) => schema.load(fs, path_id, manifest),
        }
    }
}
