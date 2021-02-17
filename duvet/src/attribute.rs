use crate::schema::Id;
use const_sha1::{sha1, ConstBuffer};
use core::{fmt, marker::PhantomData};
use sled::IVec;
use zerocopy::AsBytes;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Attribute<T> {
    key: [u8; 20],
    path: &'static str,
    value: PhantomData<T>,
}

impl<T> Attribute<T> {
    #[doc(hidden)]
    pub const fn new(path: &'static str) -> Self {
        Self {
            key: sha1(&ConstBuffer::from_slice(path.as_bytes())).bytes(),
            path,
            value: PhantomData,
        }
    }

    pub const fn dependency(&self) -> Dependency {
        Dependency {
            key: self.key,
            path: self.path,
        }
    }

    pub(crate) fn prefix_with<I: Id>(&self, id: I) -> [u8; 24] {
        let mut out = [0u8; 24];
        out[..4].copy_from_slice(id.into_inner().as_bytes());
        out[4..].copy_from_slice(&self.key);
        out
    }

    pub(crate) fn suffix_with<I: Id>(&self, id: I) -> [u8; 24] {
        let mut out = [0u8; 24];
        out[..20].copy_from_slice(&self.key);
        out[20..].copy_from_slice(id.into_inner().as_bytes());
        out
    }
}

impl<T> fmt::Debug for Attribute<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.path)
    }
}

impl<T> fmt::Display for Attribute<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.path)
    }
}

#[macro_export]
macro_rules! attribute {
    (const $name:ident : $ty:ty) => {
        const $name: $crate::attribute::Attribute<$ty> =
            $crate::attribute::Attribute::new(concat!(module_path!(), "::", stringify!($name)));
    };
    (pub(crate)const $name:ident : $ty:ty) => {
        const $name: $crate::attribute::Attribute<$ty> =
            $crate::attribute::Attribute::new(concat!(module_path!(), "::", stringify!($name)));
    };
    (pub const $name:ident : $ty:ty) => {
        pub const $name: $crate::attribute::Attribute<$ty> =
            $crate::attribute::Attribute::new(concat!(module_path!(), "::", stringify!($name)));
    };
}

pub trait Value {
    fn load(value: IVec) -> Self;
    fn store(self) -> IVec;
}

impl Value for () {
    fn load(_value: IVec) -> Self {}

    fn store(self) -> IVec {
        IVec::from(vec![])
    }
}

pub struct Dependency {
    key: [u8; 20],
    path: &'static str,
}

#[cfg(test)]
mod tests {
    attribute!(const TEST: u32);
    attribute!(pub(crate) const TEST_CRATE: u32);
    attribute!(pub const TEST_PUB: u32);

    #[test]
    fn type_test() {
        assert_eq!(TEST.to_string(), "duvet::attribute::tests::TEST");
        assert_eq!(
            TEST_CRATE.to_string(),
            "duvet::attribute::tests::TEST_CRATE"
        );
        assert_eq!(TEST_PUB.to_string(), "duvet::attribute::tests::TEST_PUB");
        assert_ne!(TEST.key, TEST_CRATE.key);
        assert_ne!(TEST.key, TEST_PUB.key);
    }
}
