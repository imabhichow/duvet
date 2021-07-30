use crate::{analyze::mapper, db::Db, report};
use core::{any::Any, fmt};
use std::hash::{Hash, Hasher};

analyzer!();

analyzer_deps!(Deps, Dep, Set);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Set {
    pub analysis: Analysis,
    pub reports: report::Map,
}

pub trait AnalyzeObj: 'static + Any + fmt::Debug + Send + Sync {
    fn dependencies(&self) -> (&[mapper::Category], &[Category]);
    fn analyze(&self, single_deps: Vec<mapper::Map>, global_deps: Vec<Set>) -> Set;
    fn dyn_eq(&self, other: &dyn AnalyzeObj) -> bool;
    fn dyn_hash(&self, hasher: &mut dyn Hasher);
}

pub trait Analyze: 'static + Eq + Hash + fmt::Debug + Send + Sync {
    type Reducers: Deps;
    type Mappers: mapper::DepsMap;
    type Output: Output;

    fn analyze(
        &self,
        mappers: Self::Mappers,
        reducers: Self::Reducers,
    ) -> (Option<Self::Output>, report::Map);
}

#[derive(Debug)]
struct StaticAnalyzer<T: Analyze> {
    analyzer: T,
    reducer_deps: Vec<Category>,
    mapper_deps: Vec<mapper::Category>,
}

impl<T: Analyze> StaticAnalyzer<T> {
    fn new(analyzer: T) -> Self {
        let reducer_deps = T::Reducers::list();
        let mapper_deps = <T::Mappers as mapper::DepsMap>::list();
        Self {
            analyzer,
            reducer_deps,
            mapper_deps,
        }
    }
}

impl<T: Analyze> AnalyzeObj for StaticAnalyzer<T> {
    fn dependencies(&self) -> (&[mapper::Category], &[Category]) {
        (&self.mapper_deps, &self.reducer_deps)
    }

    fn analyze(&self, mappers: Vec<mapper::Map>, reducers: Vec<Set>) -> Set {
        let mappers = <T::Mappers as mapper::DepsMap>::new(mappers);
        let reducers = T::Reducers::new(reducers);

        let (analysis, reports) = self.analyzer.analyze(mappers, reducers);

        Set {
            analysis: Analysis::new(analysis),
            reports,
        }
    }

    fn dyn_eq(&self, _other: &dyn AnalyzeObj) -> bool {
        todo!()
    }

    fn dyn_hash(&self, mut hasher: &mut dyn Hasher) {
        self.analyzer.hash(&mut hasher);
    }
}

pub fn reduce(db: &dyn Db, analyzer: Analyzer) -> Set {
    let (mappers, reducers) = analyzer.0.dependencies();

    let mappers = mappers.iter().map(|dep| db.map_category(*dep)).collect();

    let reducers = reducers
        .iter()
        .map(|dep| db.reduce_category(*dep))
        .collect();

    analyzer.0.analyze(mappers, reducers)
}

pub fn reduce_category(db: &dyn Db, ty: Category) -> Set {
    let manifest = db.manifest();
    let analyzers = manifest.reducers();

    if let Some(analyzer) = analyzers.get(&ty) {
        db.reduce(analyzer.clone())
    } else {
        Set {
            analysis: Analysis::empty(),
            reports: report::Map::empty(),
        }
    }
}
