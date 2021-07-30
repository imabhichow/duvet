use duvet_core::{
    database::{Database, Offline as Db},
    diagnostics,
    fs::Node,
    manifests, mapper, Fs, Manifest,
};
use std::{path::Path, sync::Arc};

struct Loader;

impl manifests::Loader for Loader {
    fn load(&self, vfs: Fs) -> Result<Manifest, diagnostics::Map> {
        let root = std::env::current_dir().unwrap();
        let root = vfs.path_to_id(&root);
        let mut manifest = Manifest::builder(root);

        manifest.with_mapper(LineCounter);

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

fn new_db() -> Db {
    let loader = Arc::new(Loader);
    Db::new(loader)
}

fn main() {
    let db = new_db();

    println!("{:#?}", Database::path_diagnostics(&db, Path::new(file!())));
}

#[test]
fn self_test() {
    let db = new_db();

    println!("{:#?}", Database::path_diagnostics(&db, Path::new(file!())));
}
