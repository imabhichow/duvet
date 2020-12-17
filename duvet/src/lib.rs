use std::collections::HashMap;

pub mod export;

pub type TypeID = u64;
pub type AnnotationID = u64;
pub type ExpansionID = u64;
pub type NotificationID = u64;
pub type InstantiationID = u64;
pub type FileID = u64;

pub struct Report {
    pub instantiations: HashMap<InstantiationID, Instantiation>,
    pub annotations: HashMap<AnnotationID, Annotation>,
    pub types: HashMap<TypeID, Type>,
    pub notifications: HashMap<NotificationID, Notification>,
    pub files: HashMap<FileID, File>,
    pub aliases: Vec<Alias>,
}

pub struct Instantiation {
    pub name: Option<String>,
    pub start: usize,
    pub end: usize,
    pub file: FileID,
}

pub struct Alias {
    pub file: FileID,
    pub aliases: Vec<FileID>,
}

pub struct File {
    pub name: Option<String>,
    pub path: String,
    pub language: Option<String>,
}

pub struct Annotation {
    pub types: Vec<TypeID>,
    pub regions: Vec<Region>,
    pub description: Option<String>,
    pub metric: Option<u64>,
    pub reasons: Vec<AnnotationID>,
}

pub struct Region {
    pub instantiation: InstantiationID,
    pub start: usize,
    pub end: usize,
}

pub struct Type {
    pub name: String,
    // The type will be fulfilled if all of the dependencies are
    pub implicit: bool,
    pub dependencies: Vec<Dependency>,
}

pub struct Dependency {
    pub notification: Option<NotificationID>,
    pub message: Option<String>,
    pub node: DependencyNode,
}

pub enum DependencyNode {
    Id(TypeID),
    Any(Vec<DependencyNode>),
    All(Vec<DependencyNode>),
}

pub struct Notification {
    pub name: Option<String>,
    pub description: Option<String>,
    pub level: Level,
}

pub enum Level {
    Fatal,
    Error,
    Warning,
    Info,
    Debug,
}
