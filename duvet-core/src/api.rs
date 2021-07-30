use std::path::{Path, PathBuf};

pub use self::{database::Database, fs::Fs, manifests::Manifest};

pub mod mapper {
    pub use crate::analyze::mapper::{Analyze, Dep, Deps, DepsMap, Map, Set};
    pub use globset::Glob;
}

pub mod database {
    use super::*;
    pub use crate::db::{offline::Offline, online::Online};

    pub trait Database {
        fn path_diagnostics(&self, path: &Path) -> diagnostics::Set;
        // TODO fn workspace_diagnostics(&self) -> diagnostics::Map;
        // TODO fn generate_reports(&self);
    }
}

pub mod diagnostics {
    pub use crate::report::{Diagnostic, List, Map, Set};
}

pub mod manifests {
    pub use crate::manifest::{BuildError, Builder, Loader, Manifest};
}

pub mod fs {
    use super::*;
    use crate::db::Db;

    pub use crate::{
        intern::Ref,
        vfs::{Node, PathId},
    };

    pub struct Fs<'a>(&'a dyn Db);

    impl<'a> Fs<'a> {
        pub(crate) fn new(db: &'a dyn Db) -> Self {
            Self(db)
        }

        pub fn read(&self, path: PathId) -> Node {
            self.0.vfs_read(path)
        }

        pub fn path_to_id(&self, path: &Path) -> PathId {
            self.0.paths().intern(path)
        }

        pub fn id_to_path(&self, id: PathId) -> Ref<PathBuf> {
            self.0.paths().resolve(id)
        }
    }
}
