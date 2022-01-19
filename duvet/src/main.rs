use duvet::{Database, Loader};
use std::path::PathBuf;

#[derive(Debug)]
enum Arguments {
    Extract(Extract),
    Report(Report),
}

#[derive(Debug)]
struct Extract {
    manifest_path: PathBuf,
}

#[derive(Debug)]
struct Report {
    // TODO
}

fn main() {
    let root = std::env::current_dir().unwrap().join("duvet.toml");
    let db = Database::new(Loader { root });
    db.report_all();
}
