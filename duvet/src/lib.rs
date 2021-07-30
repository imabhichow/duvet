use duvet_core::{database::Offline, diagnostics, Database as _};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

type Result<V, E = Error> = core::result::Result<V, E>;
type Error = anyhow::Error;

mod manifest;

pub struct Database(Offline);

impl Database {
    pub fn new(root: PathBuf) -> Self {
        let loader = Arc::new(manifest::Loader { root });
        let db = duvet_core::database::Offline::new(loader);
        Self(db)
    }

    pub fn path_diagnostics(&self, path: &Path) -> diagnostics::Set {
        self.0.path_diagnostics(path)
    }
}
