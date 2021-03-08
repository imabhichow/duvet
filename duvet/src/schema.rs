use crate::attribute::Value;
use byteorder::BigEndian as BE;
use sled::IVec;
use zerocopy::{byteorder::U32, AsBytes, FromBytes, Unaligned};

macro_rules! id {
    ($name:ident) => {
        #[derive(AsBytes, FromBytes, Unaligned, Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(C)]
        pub struct $name(pub(crate) U32<BE>);

        impl $name {
            pub(crate) fn new(value: u32) -> Self {
                Self(U32::new(value))
            }
        }

        impl Id for $name {
            fn into_inner(self) -> U32<BE> {
                self.0
            }

            fn from_inner(value: U32<BE>) -> Self {
                Self(value)
            }
        }

        impl Value for $name {
            fn load(value: IVec) -> Self {
                Self::new(Value::load(value))
            }

            fn store(self) -> IVec {
                self.into()
            }
        }

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self::new(value)
            }
        }

        impl From<U32<BE>> for $name {
            fn from(value: U32<BE>) -> Self {
                Self(value)
            }
        }

        impl AsRef<U32<BE>> for $name {
            fn as_ref(&self) -> &U32<BE> {
                &self.0
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }

        impl From<$name> for sled::IVec {
            fn from(value: $name) -> sled::IVec {
                sled::IVec::from(value.as_bytes())
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(&other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.0.get().cmp(&other.0.get())
            }
        }
    };
}

id!(SourceId);
id!(SourceLocationId);
id!(TypeId);
id!(AnnotationId);
id!(FileId);
id!(InstanceId);
id!(ExpansionId);
id!(NotificationId);
id!(InstantiationId);
id!(ReporterId);
id!(EntityId);

impl InstanceId {
    pub(crate) fn default() -> Self {
        Self::new(0)
    }
}

pub(crate) trait Id: Copy + Sized {
    fn into_inner(self) -> U32<BE>;
    fn from_inner(value: U32<BE>) -> Self;
}

impl Id for u32 {
    fn into_inner(self) -> U32<BE> {
        U32::new(self)
    }

    fn from_inner(value: U32<BE>) -> Self {
        value.get()
    }
}

impl Id for U32<BE> {
    fn into_inner(self) -> U32<BE> {
        self
    }

    fn from_inner(value: U32<BE>) -> Self {
        value
    }
}

pub(crate) trait IdSet {
    type Output: Sized + Copy + AsRef<[u8]>;

    fn join(self) -> Self::Output;
    fn keys<T: AsRef<[u8]>>(input: T) -> Self;
}

pub(crate) trait IdSetExt {
    fn keys<Set: IdSet>(&self) -> Set;
}

impl<T: AsRef<[u8]>> IdSetExt for T {
    fn keys<Set: IdSet>(&self) -> Set {
        Set::keys(self.as_ref())
    }
}

macro_rules! join_tuple {
    ($($t:ident),*) => {
        impl<$($t: Id),*> IdSet for ($($t,)*) {
            type Output = [u8; (0 $(+ stringify!($t).len())*) * 4];

            fn join(self) -> Self::Output {
                #![allow(non_snake_case)]

                let ($($t,)*) = self;

                $(
                    let $t = $t.into_inner();
                    let $t = $t.as_bytes();
                )*

                [
                    $(
                        $t[0],
                        $t[1],
                        $t[2],
                        $t[3],
                    )*
                ]
            }

            fn keys<T: AsRef<[u8]>>(input: T) -> Self {
                #![allow(non_snake_case)]

                let input = input.as_ref();

                assert_eq!(input.len(), core::mem::size_of::<Self::Output>());

                $(
                    let (v, input) = input.split_at(4);
                    let $t = $t::from_inner(U32::new(u32::from_be_bytes([v[0], v[1], v[2], v[3]])));
                )*

                debug_assert!(input.is_empty());
                let _ = input;

                (
                    $(
                        $t,
                    )*
                )
            }
        }
    };
}

join_tuple!(A);
join_tuple!(A, B);
join_tuple!(A, B, C);
join_tuple!(A, B, C, D);
join_tuple!(A, B, C, D, E);
join_tuple!(A, B, C, D, E, F);
join_tuple!(A, B, C, D, E, F, G);
