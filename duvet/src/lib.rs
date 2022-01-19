use duvet_core::{database::Offline, diagnostics, Database as _};
use std::{path::Path, sync::Arc};

type Result<V, E = Error> = core::result::Result<V, E>;
type Error = anyhow::Error;

mod analysis;
mod citation_type;
mod manifest;
mod static_intern;

pub use manifest::Loader;

pub struct Database(Offline);

impl Database {
    pub fn new(loader: manifest::Loader) -> Self {
        let loader = Arc::new(loader);
        let db = duvet_core::database::Offline::new(loader);
        Self(db)
    }

    pub fn path_diagnostics(&self, path: &Path) -> diagnostics::MultiList {
        self.0.path_diagnostics(path)
    }

    pub fn report_all(&self) -> diagnostics::Map {
        self.0.report_all()
    }
}
