use duvet_core::intern::{self, Intern};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
struct Types(Arc<Intern<String>>);

impl Types {
    pub fn intern(&self, name: &str) -> Id {
        Id(self.0.intern(name))
    }

    pub fn resolve(&self, id: Id) -> intern::Ref<String> {
        self.0.resolve(id.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(intern::Id);
