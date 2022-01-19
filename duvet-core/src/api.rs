use std::path::{Path, PathBuf};

pub use self::{database::Database, fs::Fs, manifests::Manifest};

pub mod mapper {
    pub use crate::analyze::mapper::{Analyze, Dep, DepMap, Deps, DepsMap, Map, Set};
    pub use globset::Glob;
}

pub mod reducer {
    pub use crate::analyze::reducer::{Analyze, Dep, Deps, Set};
}

pub mod reporter {
    pub use crate::analyze::reporter::Analyze;
}

pub mod database {
    use super::*;
    pub use crate::db::{offline::Offline, online::Online};

    pub trait Database {
        fn path_diagnostics(&self, path: &Path) -> diagnostics::MultiList;
        // TODO fn workspace_diagnostics(&self) -> diagnostics::Map;
        // TODO fn generate_reports(&self);
        fn report_all(&self) -> diagnostics::Map;
    }
}

pub mod diagnostics {
    pub use crate::report::{Diagnostic, List, Map, MultiList};
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
    pub use arcstr::{self, ArcStr, Substr};

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
