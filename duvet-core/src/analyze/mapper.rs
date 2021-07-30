use crate::{
    analyze::reducer,
    db::Db,
    report,
    vfs::{Node, PathId, PathIdMap},
};
use core::{any::Any, fmt, ops::Deref};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::{
    hash::{Hash, Hasher},
    path::Path,
};

analyzer!();

pub trait AnalyzeObj: 'static + Any + fmt::Debug + Send + Sync {
    fn is_match(&self, path: &Path) -> bool;
    fn dependencies(&self) -> (&[Category], &[reducer::Category]);
    fn analyze(
        &self,
        single_deps: Vec<Set>,
        global_deps: Vec<reducer::Set>,
        path: &Path,
        node: Node,
    ) -> Set;
    fn dyn_eq(&self, other: &dyn AnalyzeObj) -> bool;
    fn dyn_hash(&self, hasher: &mut dyn Hasher);
}

pub trait Analyze: 'static + Eq + Hash + fmt::Debug + Send + Sync {
    type Reducers: reducer::Deps;
    type Mappers: Deps;
    type Output: Output;

    fn patterns(&self) -> Vec<Glob>;

    fn analyze(
        &self,
        mappers: Self::Mappers,
        reducers: Self::Reducers,
        path: &Path,
        node: Node,
    ) -> (Option<Self::Output>, report::List);
}

#[derive(Debug)]
struct StaticAnalyzer<T: Analyze> {
    analyzer: T,
    reducer_deps: Vec<reducer::Category>,
    mapper_deps: Vec<Category>,
    patterns: GlobSet,
}

impl<T: Analyze> StaticAnalyzer<T> {
    fn new(analyzer: T) -> Self {
        let reducer_deps = <T::Reducers as reducer::Deps>::list();
        let mapper_deps = T::Mappers::list();
        let mut patterns = GlobSetBuilder::new();

        for pattern in analyzer.patterns() {
            patterns.add(pattern);
        }

        Self {
            analyzer,
            reducer_deps,
            mapper_deps,
            patterns: patterns.build().unwrap(),
        }
    }
}

impl<T: Analyze> AnalyzeObj for StaticAnalyzer<T> {
    fn is_match(&self, path: &Path) -> bool {
        self.patterns.is_match(path)
    }

    fn dependencies(&self) -> (&[Category], &[reducer::Category]) {
        (&self.mapper_deps, &self.reducer_deps)
    }

    fn analyze(
        &self,
        mappers: Vec<Set>,
        reducers: Vec<reducer::Set>,
        path: &Path,
        node: Node,
    ) -> Set {
        let mappers = T::Mappers::new(mappers);
        let reducers = <T::Reducers as reducer::Deps>::new(reducers);

        let (analysis, report) = self.analyzer.analyze(mappers, reducers, path, node);

        Set {
            analysis: Analysis::new(analysis),
            report,
        }
    }

    fn dyn_eq(&self, _other: &dyn AnalyzeObj) -> bool {
        todo!()
    }

    fn dyn_hash(&self, mut hasher: &mut dyn Hasher) {
        self.analyzer.hash(&mut hasher);
    }
}

pub fn map_path(db: &dyn Db, path_id: PathId, analyzer: Analyzer) -> Set {
    let paths = db.paths();
    let path = paths.resolve(path_id);
    let node = db.vfs_read(path_id);

    let (mappers, reducers) = analyzer.0.dependencies();

    let mappers = mappers
        .iter()
        .map(|dep| db.map_path_category(path_id, *dep))
        .collect();

    let reducers = reducers
        .iter()
        .map(|dep| db.reduce_category(*dep))
        .collect();

    analyzer.0.analyze(mappers, reducers, &*path, node)
}

pub fn map_path_category(db: &dyn Db, path_id: PathId, ty: Category) -> Set {
    let manifest = db.manifest();
    let analyzers = manifest.mappers();
    if let Some(analyzer) = analyzers.get(&ty) {
        db.map_path(path_id, analyzer.clone())
    } else {
        Set {
            analysis: Analysis::empty(),
            report: report::List::empty(),
        }
    }
}

pub fn map_category(db: &dyn Db, ty: Category) -> Map {
    let sources = db.mapper_sources(ty);

    let paths = sources
        .iter()
        .copied()
        .map(|source| {
            let set = db.map_path_category(source, ty);
            (source, set)
        })
        .collect();

    Map { paths }
}

analyzer_deps!(Deps, Dep, Set);

impl<T: Output> Dep<T> {
    fn get(dep: &Self) -> Option<&T> {
        Some(
            dep.inner
                .analysis
                .0
                .as_ref()?
                .as_any()
                .downcast_ref()
                .unwrap(),
        )
    }
}

impl<T: Output> Deref for Dep<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Self::get(self)
            .unwrap_or_else(|| panic!("missing dependency {}", core::any::type_name::<T>()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Set {
    pub analysis: Analysis,
    pub report: report::List,
}

analyzer_deps!(DepsMap, DepMap, Map);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    paths: PathIdMap<Set>,
}
