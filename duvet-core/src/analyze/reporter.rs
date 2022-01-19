use crate::{
    analyze::{mapper, reducer},
    db::Db,
    report,
};
use core::{any::Any, fmt};
use std::hash::{Hash, Hasher};

analyzer!();

pub trait AnalyzeObj: 'static + Any + fmt::Debug + Send + Sync {
    fn dependencies(&self) -> (&[mapper::Category], &[reducer::Category]);
    fn analyze(&self, single_deps: Vec<mapper::Map>, global_deps: Vec<reducer::Set>)
        -> report::Map;
    fn dyn_eq(&self, other: &dyn AnalyzeObj) -> bool;
    fn dyn_hash(&self, hasher: &mut dyn Hasher);
}

pub trait Analyze: 'static + Eq + Hash + fmt::Debug + Send + Sync {
    type Reducers: reducer::Deps;
    type Mappers: mapper::DepsMap;

    fn analyze(&self, mappers: Self::Mappers, reducers: Self::Reducers) -> report::Map;
}

#[derive(Debug)]
struct StaticAnalyzer<T: Analyze> {
    analyzer: T,
    reducer_deps: Vec<reducer::Category>,
    mapper_deps: Vec<mapper::Category>,
}

impl<T: Analyze> StaticAnalyzer<T> {
    fn new(analyzer: T) -> Self {
        let reducer_deps = <T::Reducers as reducer::Deps>::list();
        let mapper_deps = <T::Mappers as mapper::DepsMap>::list();
        Self {
            analyzer,
            reducer_deps,
            mapper_deps,
        }
    }
}

impl<T: Analyze> AnalyzeObj for StaticAnalyzer<T> {
    fn dependencies(&self) -> (&[mapper::Category], &[reducer::Category]) {
        (&self.mapper_deps, &self.reducer_deps)
    }

    fn analyze(&self, mappers: Vec<mapper::Map>, reducers: Vec<reducer::Set>) -> report::Map {
        let mappers = <T::Mappers as mapper::DepsMap>::new(mappers);
        let reducers = <T::Reducers as reducer::Deps>::new(reducers);

        self.analyzer.analyze(mappers, reducers)
    }

    fn dyn_eq(&self, _other: &dyn AnalyzeObj) -> bool {
        todo!()
    }

    fn dyn_hash(&self, mut hasher: &mut dyn Hasher) {
        self.analyzer.hash(&mut hasher);
    }
}

pub fn report(db: &dyn Db, analyzer: Analyzer) -> report::Map {
    let (mappers, reducers) = analyzer.0.dependencies();

    let mappers = mappers.iter().map(|dep| db.map_category(*dep)).collect();

    let reducers = reducers
        .iter()
        .map(|dep| db.reduce_category(*dep))
        .collect();

    analyzer.0.analyze(mappers, reducers)
}

pub fn report_all(db: &dyn Db) -> report::Map {
    let manifest = db.manifest();
    let reporters = manifest.reporters();

    for analyzer in reporters {
        // TODO merge reports into multi map
        let _report = db.report(analyzer.clone());
    }

    report::Map::empty()
}
