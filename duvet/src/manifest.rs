use duvet_core::{diagnostics, fs::Node, manifests, Fs, Manifest};
use std::path::PathBuf;

mod schema;

#[derive(Clone, Debug)]
pub struct Loader {
    pub root: PathBuf,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().expect("could not fetch current_dir"),
        }
    }
}

impl manifests::Loader for Loader {
    fn load(&self, fs: Fs) -> Result<Manifest, diagnostics::Map> {
        let root_id = fs.path_to_id(&self.root);

        let mut manifest = Manifest::builder(root_id);

        match fs.read(root_id) {
            Node::String(_, contents) => {
                let schema = schema::Schema::parse(&self.root, &contents.to_string())
                    .expect("TODO convert this");
                schema.load(&fs, root_id, &mut manifest);
            }
            // TODO load multiple
            other => todo!("{:?}", other),
        }

        let manifest = manifest
            .build()
            .expect("TODO convert this into diagnostics");

        Ok(manifest)
    }
}
