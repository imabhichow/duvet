use crate::Result;
use std::path::Path;

pub mod v1;

pub fn parse(file: &Path, contents: &str) -> Result<v1::Manifest> {
    match file.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            // TODO add version entry
            let manifest = toml::from_str(contents)?;
            Ok(manifest)
        }
        ext => unimplemented!("{:?}", ext),
    }
}
