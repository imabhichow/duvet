use std::path::Path;

use duvet::Database;

fn main() {
    let manifest = std::env::current_dir().unwrap().join("duvet.toml");
    let db = Database::new(manifest);
    eprintln!("{:#?}", db.path_diagnostics(Path::new(file!())));
}
