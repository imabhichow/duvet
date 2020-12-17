use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub packages: Vec<Package>,
    pub workspace_root: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub manifest_path: String,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub kind: Vec<String>,
    pub name: String,
    pub src_path: String,
    pub test: Option<bool>,
}
