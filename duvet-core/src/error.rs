use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Error(Arc<String>);

impl<T: core::fmt::Display> From<T> for Error {
    fn from(value: T) -> Self {
        Self(Arc::new(value.to_string()))
    }
}

pub type Result<V, E = Error> = core::result::Result<V, E>;
