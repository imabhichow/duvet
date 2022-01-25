use crate::citation::{tree::Tree, types::Level};

#[derive(Clone, Debug)]
pub struct Requirement {
    level: Level,
    tree: Tree,
}
