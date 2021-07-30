use crate::{
    analyze::{mapper, reducer},
    db::Db,
    vfs::PathId,
};
use core::ops::Deref;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::{
    collections::{btree_map::Entry, BTreeMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Manifest(Arc<Inner>);

impl Manifest {
    fn patterns(&self) -> GlobSet {
        let mut builder = GlobSetBuilder::new();
        for glob in &self.0.patterns {
            builder.add(glob.clone());
        }
        builder.build().unwrap()
    }

    pub(crate) fn mappers(&self) -> &BTreeMap<mapper::Category, mapper::Analyzer> {
        &self.0.mappers
    }

    pub(crate) fn reducers(&self) -> &BTreeMap<reducer::Category, reducer::Analyzer> {
        &self.0.reducers
    }

    pub fn builder(root: PathId) -> Builder {
        Builder::new(root)
    }
}

#[derive(Clone, Debug)]
pub struct Builder(Inner);

impl Builder {
    pub fn new(root: PathId) -> Self {
        Builder(Inner {
            patterns: vec![],
            mappers: Default::default(),
            reducers: Default::default(),
            root,
        })
    }

    pub fn with_mapper<T: mapper::Analyze>(&mut self, analyzer: T) -> &mut Self {
        let category = mapper::Category::of::<T::Output>();

        // Add the mapper's patterns to the globals
        self.0.patterns.extend(analyzer.patterns());

        match self.0.mappers.entry(category) {
            Entry::Vacant(entry) => {
                entry.insert(mapper::Analyzer::new(analyzer));
            }
            Entry::Occupied(prev) => {
                panic!(
                    "mapper category {:?} is already fulfilled by {:?}",
                    category, prev,
                );
            }
        }

        self
    }

    pub fn with_reducer<T: reducer::Analyze>(&mut self, analyzer: T) -> &mut Self {
        let category = reducer::Category::of::<T::Output>();

        match self.0.reducers.entry(category) {
            Entry::Vacant(entry) => {
                entry.insert(reducer::Analyzer::new(analyzer));
            }
            Entry::Occupied(prev) => {
                panic!(
                    "reducer category {:?} is already fulfilled by {:?}",
                    category, prev,
                );
            }
        }

        self
    }

    pub fn build(self) -> Result<Manifest, BuildError> {
        let mut error = BuildError::new();

        for (_category, mapper) in self.0.mappers.iter() {
            // TODO build a graph and make sure it's acyclical
            let (mapper_deps, reducer_deps) = mapper.dependencies();

            for dep in mapper_deps {
                if !self.0.mappers.contains_key(dep) {
                    error.missing_mappers.insert(*dep);
                }
            }

            for dep in reducer_deps {
                if !self.0.reducers.contains_key(dep) {
                    error.missing_reducers.insert(*dep);
                }
            }
        }

        for (_category, reducer) in self.0.reducers.iter() {
            // TODO build a graph and make sure it's acyclical
            let (mapper_deps, reducer_deps) = reducer.dependencies();

            for dep in mapper_deps {
                if !self.0.mappers.contains_key(dep) {
                    error.missing_mappers.insert(*dep);
                }
            }

            for dep in reducer_deps {
                if !self.0.reducers.contains_key(dep) {
                    error.missing_reducers.insert(*dep);
                }
            }
        }

        if error.is_empty() {
            Ok(Manifest(Arc::new(self.0)))
        } else {
            Err(error)
        }
    }
}

#[derive(Debug)]
pub struct BuildError {
    missing_reducers: HashSet<reducer::Category>,
    missing_mappers: HashSet<mapper::Category>,
}

impl BuildError {
    fn new() -> Self {
        Self {
            missing_reducers: HashSet::new(),
            missing_mappers: HashSet::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.missing_mappers.is_empty() && self.missing_reducers.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Inner {
    patterns: Vec<Glob>,
    mappers: BTreeMap<mapper::Category, mapper::Analyzer>,
    reducers: BTreeMap<reducer::Category, reducer::Analyzer>,
    root: PathId,
}

pub trait Loader {
    fn load(&self, fs: crate::api::Fs<'_>) -> Result<Manifest, crate::report::Map>;
}

pub trait DbLoader {
    fn manifest_loader(&self) -> &dyn Loader;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sources(Arc<[PathId]>);

impl Deref for Sources {
    type Target = [PathId];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

pub fn manifest(db: &dyn Db) -> Manifest {
    let loader = db.manifest_loader();
    let result = loader.load(crate::api::Fs::new(db));

    result.unwrap_or_else(|_| todo!("implement diagnostics"))
}

pub fn manifest_sources(db: &dyn Db) -> Sources {
    let manifest = db.manifest();
    let patterns = manifest.patterns();

    let paths = db.paths();
    let root = paths.resolve(manifest.0.root);

    // TODO implement a walker against the virtual file system instead
    let mut sources = vec![];
    for entry in ignore::WalkBuilder::new(&*root)
        .build()
        .flat_map(|v| v.ok())
    {
        if patterns.is_match(entry.path()) {
            sources.push(paths.intern(entry.path()));
        }
    }

    Sources(Arc::from(sources.into_boxed_slice()))
}

pub fn mapper_sources(db: &dyn Db, ty: mapper::Category) -> Sources {
    let manifest = db.manifest();
    let sources = db.manifest_sources();
    let paths = db.paths();

    let mut results = vec![];

    if let Some(parser) = manifest.mappers().get(&ty) {
        for source in sources.iter().copied() {
            let path = paths.resolve(source);

            if parser.is_match(&*path) {
                results.push(source);
            }
        }
    }

    Sources(Arc::from(results.into_boxed_slice()))
}
