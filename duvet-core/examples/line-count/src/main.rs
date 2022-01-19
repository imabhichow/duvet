use duvet_core::{
    database::{Database, Offline as Db},
    diagnostics,
    fs::Node,
    manifests, mapper, reporter, Fs, Manifest,
};
use std::{path::Path, sync::Arc};

struct Loader;

impl manifests::Loader for Loader {
    fn load(&self, vfs: Fs) -> Result<Manifest, diagnostics::Map> {
        let root = std::env::current_dir().unwrap();
        let root = vfs.path_to_id(&root);
        let mut manifest = Manifest::builder(root);

        manifest.with_mapper(LineCounter);
        manifest.with_reporter(LineReport);

        let manifest = manifest.build().unwrap();
        Ok(manifest)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct LineCount(usize);

#[derive(Debug, PartialEq, Eq, Hash)]
struct LineCounter;

impl mapper::Analyze for LineCounter {
    type Reducers = ();
    type Mappers = ();
    type Output = LineCount;

    fn patterns(&self) -> Vec<mapper::Glob> {
        vec![mapper::Glob::new("**/*.rs").unwrap()]
    }

    fn analyze(
        &self,
        _reducers: (),
        _mappers: (),
        _path: &Path,
        node: Node,
    ) -> (Option<LineCount>, diagnostics::List) {
        if let Ok(contents) = node.as_str() {
            let count = contents.lines().count();
            (Some(LineCount(count)), diagnostics::List::empty())
        } else {
            // TODO add error
            (None, diagnostics::List::empty())
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct LineReport;

impl reporter::Analyze for LineReport {
    type Reducers = ();
    type Mappers = (mapper::DepMap<LineCount>,);

    fn analyze(
        &self,
        (line_counts,): Self::Mappers,
        _reducers: Self::Reducers,
    ) -> diagnostics::Map {
        for (file, counts) in line_counts.iter() {
            println!("{}: {}", file.display(), counts.0);
        }

        Default::default()
    }
}

fn new_db() -> Db {
    let loader = Arc::new(Loader);
    Db::new(loader)
}

fn main() {
    new_db().report_all();
}

#[test]
fn self_test() {
    new_db().report_all();
}
