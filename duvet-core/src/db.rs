use crate::{
    analyze::{
        mapper::{self, map_category, map_path, map_path_category},
        reducer::{self, reduce, reduce_category},
        reporter::{self, report, report_all},
    },
    api,
    manifest::{self, manifest, manifest_sources, mapper_sources, Manifest},
    report::{diagnose_path, Map as ReportMap, MultiList},
    vfs::{self, vfs_read},
};

#[salsa::query_group(Storage)]
pub trait Db: salsa::Database + vfs::Filesystem + manifest::DbLoader {
    /// Returns the workspace's manifest
    fn manifest(&self) -> Manifest;

    /// Returns all of the sources contained in the manifest
    fn manifest_sources(&self) -> manifest::Sources;

    /// Returns all of the sources for a given mapper category
    fn mapper_sources(&self, category: mapper::Category) -> manifest::Sources;

    fn map_path(&self, path: vfs::PathId, analyzer: mapper::Analyzer) -> mapper::Set;

    fn map_path_category(&self, path: vfs::PathId, ty: mapper::Category) -> mapper::Set;

    fn map_category(&self, mapper::ty: mapper::Category) -> mapper::Map;

    fn reduce(&self, analyzer: reducer::Analyzer) -> reducer::Set;

    fn reduce_category(&self, ty: reducer::Category) -> reducer::Set;

    fn report(&self, analyzer: reporter::Analyzer) -> ReportMap;

    fn report_all(&self) -> ReportMap;

    fn diagnose_path(&self, path: vfs::PathId) -> MultiList;

    /// Reads a file from the file system
    fn vfs_read(&self, path: vfs::PathId) -> vfs::Node;
}

pub mod offline {
    use super::*;
    use std::{path::Path, sync::Arc};

    pub struct Offline(Inner);

    impl Offline {
        pub fn new(loader: Arc<dyn manifest::Loader>) -> Self {
            Self(Inner {
                storage: Default::default(),
                paths: Default::default(),
                loader,
            })
        }

        pub fn did_change(&mut self, path: &Path) {
            let path = self.0.paths.intern(path);
            VfsReadQuery.in_db_mut(&mut self.0).invalidate(&path);
        }
    }

    impl api::Database for Offline {
        fn path_diagnostics(&self, path: &Path) -> api::diagnostics::MultiList {
            self.0.path_diagnostics(path)
        }

        fn report_all(&self) -> api::diagnostics::Map {
            api::Database::report_all(&self.0)
        }
    }

    #[salsa::database(Storage)]
    struct Inner {
        storage: salsa::Storage<Self>,
        paths: vfs::Paths,
        loader: Arc<dyn manifest::Loader>,
    }

    impl salsa::Database for Inner {}

    impl api::Database for Inner {
        fn path_diagnostics(&self, path: &Path) -> api::diagnostics::MultiList {
            let path = self.paths.intern(path);
            self.diagnose_path(path)
        }

        fn report_all(&self) -> crate::diagnostics::Map {
            Db::report_all(self)
        }
    }

    impl manifest::DbLoader for Inner {
        fn manifest_loader(&self) -> &dyn manifest::Loader {
            self.loader.as_ref()
        }
    }

    impl vfs::Filesystem for Inner {
        fn paths(&self) -> &vfs::Paths {
            &self.paths
        }
    }
}

pub mod online {
    use super::*;
    use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
    use std::{
        path::Path,
        sync::{Arc, Mutex},
    };

    pub struct Online(Inner);

    impl Online {
        pub fn new(
            loader: Arc<dyn manifest::Loader>,
            watcher: Arc<Mutex<RecommendedWatcher>>,
        ) -> Self {
            Self(Inner {
                storage: Default::default(),
                paths: Default::default(),
                loader,
                watcher,
            })
        }

        pub fn on_event(&mut self, event: DebouncedEvent) {
            match event {
                DebouncedEvent::Create(path) => self.did_change(&path),
                DebouncedEvent::Write(path) => self.did_change(&path),
                DebouncedEvent::Chmod(path) => self.did_change(&path),
                DebouncedEvent::Remove(path) => self.did_change(&path),
                DebouncedEvent::Rename(from, to) => {
                    self.did_change(&from);
                    self.did_change(&to);
                }
                _ => {}
            }
        }

        pub fn did_change(&mut self, path: &Path) {
            let path = self.0.paths.intern(path);
            VfsReadQuery.in_db_mut(&mut self.0).invalidate(&path);
        }
    }

    impl api::Database for Online {
        fn path_diagnostics(&self, path: &Path) -> api::diagnostics::MultiList {
            self.0.path_diagnostics(path)
        }

        fn report_all(&self) -> crate::diagnostics::Map {
            api::Database::report_all(&self.0)
        }
    }

    #[salsa::database(Storage)]
    pub struct Inner {
        storage: salsa::Storage<Self>,
        paths: vfs::Paths,
        watcher: Arc<Mutex<RecommendedWatcher>>,
        loader: Arc<dyn manifest::Loader>,
    }

    impl salsa::Database for Inner {}

    impl api::Database for Inner {
        fn path_diagnostics(&self, path: &Path) -> api::diagnostics::MultiList {
            let path = self.paths.intern(path);
            self.diagnose_path(path)
        }

        fn report_all(&self) -> crate::diagnostics::Map {
            Db::report_all(self)
        }
    }

    impl manifest::DbLoader for Inner {
        fn manifest_loader(&self) -> &dyn manifest::Loader {
            self.loader.as_ref()
        }
    }

    impl vfs::Filesystem for Inner {
        fn paths(&self) -> &vfs::Paths {
            &self.paths
        }

        fn fs_watch(&self, path: &Path) {
            self.watcher
                .lock()
                .unwrap()
                .watch(path, RecursiveMode::Recursive)
                .unwrap();
        }
    }
}
