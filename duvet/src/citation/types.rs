use super::idset::IdSet;

static_intern!(Types, Type, u8);

#[derive(Clone, Copy, Debug)]
pub struct TypeSet(IdSet);

impl TypeSet {
    pub fn enable(&mut self, ty: Type) {
        self.set(ty, true);
    }

    pub fn disable(&mut self, ty: Type) {
        self.set(ty, false);
    }

    pub fn set(&mut self, ty: Type, enabled: bool) {
        self.0.set(ty.0, enabled)
    }

    pub fn get(&self, ty: Type) -> bool {
        self.0.get(ty.0)
    }
}

impl core::iter::FromIterator<Type> for TypeSet {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        Self(iter.into_iter().map(|v| v.0).collect())
    }
}

static_intern!(Levels, Level, u8);
