macro_rules! analyzer {
    () => {
        #[derive(Clone, Debug)]
        pub struct Analyzer(std::sync::Arc<dyn AnalyzeObj>);

        impl Analyzer {
            pub(crate) fn new<T: Analyze>(analyzer: T) -> Self {
                Self(std::sync::Arc::new(StaticAnalyzer::new(analyzer)))
            }
        }

        impl PartialEq for Analyzer {
            fn eq(&self, other: &Self) -> bool {
                self.0.dyn_eq(other.0.as_ref())
            }
        }

        impl Eq for Analyzer {}

        impl core::hash::Hash for Analyzer {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.0.dyn_hash(state)
            }
        }

        impl core::ops::Deref for Analyzer {
            type Target = dyn AnalyzeObj;

            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }
    };
}

macro_rules! analysis {
    () => {
        #[derive(Clone, Debug)]
        pub struct Analysis(Option<std::sync::Arc<dyn crate::analyze::Output>>);

        impl Analysis {
            pub fn empty() -> Self {
                Self(None)
            }

            pub(crate) fn new<T: crate::analyze::Output>(value: Option<T>) -> Self {
                if let Some(value) = value {
                    Self(Some(std::sync::Arc::new(value)))
                } else {
                    Self(None)
                }
            }
        }

        impl PartialEq for Analysis {
            fn eq(&self, other: &Self) -> bool {
                match (self.0.as_ref(), other.0.as_ref()) {
                    (Some(a), Some(b)) => a.dyn_eq(b.as_ref()),
                    _ => false,
                }
            }
        }

        impl Eq for Analysis {}

        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
        pub struct Category(core::any::TypeId);

        impl Category {
            pub fn of<T: crate::analyze::Output>() -> Self {
                Self(core::any::TypeId::of::<T>())
            }
        }
    };
}

macro_rules! analyzer_deps {
    ($name:ident, $dep:ident, $inner:ty) => {
        pub trait $name {
            fn list() -> Vec<Category>;
            fn new(deps: Vec<$inner>) -> Self;
        }

        impl $name for () {
            fn list() -> Vec<Category> {
                vec![]
            }

            fn new(_: Vec<$inner>) {}
        }

        impl<A: crate::analyze::Output> $name for ($dep<A>,) {
            fn list() -> Vec<Category> {
                vec![Category::of::<A>()]
            }

            fn new(mut els: Vec<$inner>) -> ($dep<A>,) {
                let mut els = els.drain(..);
                let a = $dep::new(els.next().unwrap());
                (a,)
            }
        }

        impl<A: crate::analyze::Output, B: crate::analyze::Output> $name for ($dep<A>, $dep<B>) {
            fn list() -> Vec<Category> {
                vec![Category::of::<A>(), Category::of::<B>()]
            }

            fn new(mut els: Vec<$inner>) -> ($dep<A>, $dep<B>) {
                let mut els = els.drain(..);
                let a = $dep::new(els.next().unwrap());
                let b = $dep::new(els.next().unwrap());
                (a, b)
            }
        }

        pub struct $dep<T: crate::analyze::Output> {
            #[allow(dead_code)]
            inner: $inner,
            _t: core::marker::PhantomData<T>,
        }

        impl<T: crate::analyze::Output> $dep<T> {
            fn new(inner: $inner) -> Self {
                Self {
                    inner,
                    _t: Default::default(),
                }
            }
        }
    };
}

pub trait Output: 'static + core::any::Any + core::fmt::Debug + Send + Sync {
    fn dyn_eq(&self, other: &dyn Output) -> bool;
    fn as_any(&self) -> &dyn core::any::Any;
}

impl<T: 'static + core::any::Any + core::fmt::Debug + Eq + Send + Sync> Output for T {
    fn dyn_eq(&self, other: &dyn Output) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            self.eq(other)
        } else {
            false
        }
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self as &dyn core::any::Any
    }
}

pub mod mapper;
pub mod reducer;
pub mod reporter;
