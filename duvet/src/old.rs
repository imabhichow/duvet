use core::convert::TryInto;
use hash_hasher::HashedMap;
use sha2::{Digest, Sha256};

#[derive(Default)]
struct Hasher(Sha256);

impl core::hash::Hasher for Hasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(&self) -> u64 {
        panic!("finish should not be called")
    }
}

pub type Id = u128;
pub type TypeId = Id;
pub type AnnotationId = Id;
pub type ExpansionId = Id;
pub type NotificationId = Id;
pub type InstantiationId = Id;
pub type FileId = Id;

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub instantiations: Set<Instantiation>,
    pub annotations: Set<Annotation>,
    pub types: Set<Type>,
    pub notifications: Set<Notification>,
    pub files: Set<File>,
    pub aliases: Set<Alias>,
}

#[derive(Clone, Debug, Default)]
pub struct Set<T: SetEntry>(HashedMap<Id, T>);

pub trait SetEntry: core::hash::Hash {
    fn merge(&mut self, other: Self);
}

impl<T: SetEntry> Set<T> {
    pub fn insert(&mut self, value: T) -> Id {
        let mut hasher = Hasher::default();
        value.hash(&mut hasher);
        let id = hasher.0.finalize();
        let id = (id[..core::mem::size_of::<u128>()]).try_into().unwrap();
        let id = u128::from_le_bytes(id);
        self.insert_kv(id, value);
        id
    }

    fn insert_kv(&mut self, id: Id, value: T) {
        use std::collections::hash_map::Entry;

        match self.0.entry(id) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().merge(value);
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    }

    pub fn get(&self, id: &Id) -> Option<&T> {
        self.0.get(id)
    }

    pub fn merge(&mut self, mut other: Self) {
        for (key, value) in other.0.drain() {
            self.insert_kv(key, value);
        }
    }
}

impl Report {
    pub fn merge(&mut self, other: Report) {
        self.instantiations.merge(other.instantiations);
        self.annotations.merge(other.annotations);
        self.types.merge(other.types);
        self.notifications.merge(other.notifications);
        self.files.merge(other.files);
        self.aliases.merge(other.aliases);
    }
}

/// Marks a region of text as instantiated
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Instantiation {
    /// Optional name of the instantiation
    pub name: Option<String>,
    /// Target bytes of the instantiation
    pub range: Byterange,
    /// Target file of the instantiation
    pub file: FileId,
}

impl SetEntry for Instantiation {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// Indicates that several files actually point to the same one
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Alias {
    /// Target file
    pub file: FileId,
    /// A list of aliases that point to the target file
    pub aliases: Vec<FileId>,
}

impl SetEntry for Alias {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// General file information
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct File {
    /// Optional name for the file. This will default to the basename
    pub name: Option<String>,
    /// Path to the file
    pub path: String,
    /// Language of the file.
    pub language: Option<String>,
}

impl SetEntry for File {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// Annotates a section of text
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Annotation {
    /// A set of annotation types associated with this annotation
    pub types: Vec<TypeId>,
    /// A set of regions associated with this annotation
    pub regions: Vec<Region>,
    /// A description of the annotation
    pub description: Option<String>,
    /// An optional metric associated with this annotation
    pub metric: Option<u64>,
    /// A list of annotations that caused this annotation to occur
    pub reasons: Vec<AnnotationId>,
}

impl SetEntry for Annotation {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// Associates an instantiation with a subset of bytes
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Region {
    /// The target instantiation
    pub instantiation: InstantiationId,
    /// The subset of bytes
    pub range: Byterange,
}

/// Defines the charactaristics of a particular type of annotation
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Type {
    /// The name of the type
    pub name: String,
    /// The type will be fulfilled if all of the dependencies are
    pub transparent: bool,
    /// A list of types that this type requires
    pub dependencies: Vec<Dependency>,
}

impl SetEntry for Type {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// A dependency of an annotation type
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Dependency {
    /// Include a notification if this dependency is not met
    pub notification: Option<NotificationId>,
    /// Include a description of the dependency
    pub description: Option<String>,
    /// An expression of annotation types that must be met
    pub target: DependencyTarget,
}

/// A target expression for dependencies
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum DependencyTarget {
    /// A single type
    Id(TypeId),
    /// If any of the types are met, the dependency is ok
    Any(Vec<DependencyTarget>),
    /// If all of the types are met, the dependency is ok
    All(Vec<DependencyTarget>),
}

/// A notification that is displayed when dependencies are not met
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Notification {
    /// The title of the notification
    pub title: Option<String>,
    /// The description of the notification
    pub message: Option<String>,
    /// The severity level of the notification
    pub level: Level,
}

impl SetEntry for Notification {
    fn merge(&mut self, other: Self) {
        check_collision(self, &other)
    }
}

/// A non-exlusive single region of bytes
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Byterange {
    /// Start offset
    start: usize,
    /// End offset
    end: usize,
}

/// A severity level for a notification
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Level {
    Fatal,
    Error,
    Warning,
    Info,
    Debug,
}

impl Default for Level {
    fn default() -> Self {
        Self::Info
    }
}

fn check_collision<T: PartialEq>(a: &T, b: &T) {
    if cfg!(debug_assertions) && a != b {
        panic!("hash collision detected!");
    }
}
