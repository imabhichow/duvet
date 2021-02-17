use anyhow::Result;
use duvet::db::Db;
use std::{collections::HashMap, sync::Mutex};
use tokio::task::spawn_blocking;

mod manifest;
mod process;
mod project;
mod test;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Db::new()?;
    let project = project::Builder::default().build()?;
    project.install_llvm_tools()?;
    let tests = test::list::List::from_project(&project)?;

    let progress = prodash::TreeOptions::default().create();

    let mut tasks = Tasks::new(progress.add_child("tests"));

    let is_atty = atty::is(atty::Stream::Stdout);

    let tui = if is_atty {
        tokio::spawn(prodash::render::tui(
            std::io::stdout(),
            progress,
            Default::default(),
        )?)
    } else {
        spawn_blocking(move || {
            prodash::render::line(
                std::io::stdout(),
                progress,
                prodash::render::line::Options {
                    colored: false,
                    output_is_terminal: false,
                    keep_running_if_progress_is_empty: false,
                    ..Default::default()
                },
            )
            .wait();
        })
    };

    let batch = spawn_blocking(move || -> Result<Db> {
        for test in tests.as_slice() {
            tasks.insert(test);
        }

        tests.run(|test| {
            let mut export: duvet::llvm_coverage::Export = test.run(&project)?;

            export.trim();

            tasks.with(test.id(), |task| {
                task.inc();
            });

            export.load(&db)?;

            tasks.with(test.id(), |task| {
                task.inc();
                // we don't need all of the logging in the tui
                if !is_atty {
                    task.done("✓");
                }
            });

            Ok(())
        })?;

        duvet::rust_src::RustSrc::default().report(&db)?;

        db.finish()?;

        Ok(db)
    });

    tokio::pin!(batch);
    tokio::pin!(tui);

    tokio::select! {
        _ = &mut tui => {
            std::process::exit(1);
        }
        _ = &mut batch => {
        }
    }

    Ok(())
}

struct Tasks {
    root: prodash::tree::Item,
    binaries: HashMap<String, prodash::tree::Item>,
    tests: HashMap<usize, Mutex<prodash::tree::Item>>,
}

impl Tasks {
    fn new(root: prodash::tree::Item) -> Self {
        Self {
            root,
            binaries: HashMap::new(),
            tests: HashMap::new(),
        }
    }

    fn insert(&mut self, test: &test::list::Test) {
        let root = &mut self.root;

        let binary = self
            .binaries
            .entry(test.binary().to_string())
            .or_insert_with(|| {
                root.add_child(
                    test.binary()
                        .split('/')
                        .last()
                        .unwrap()
                        .split('-')
                        .next()
                        .unwrap(),
                )
            });

        let mut task = binary.add_child(test.name());
        task.init(Some(2), None);
        self.tests.insert(test.id(), Mutex::new(task));
    }

    fn get(&self, id: usize) -> std::sync::MutexGuard<prodash::tree::Item> {
        self.tests.get(&id).unwrap().lock().unwrap()
    }

    fn with<F: FnOnce(&mut prodash::tree::Item)>(&self, id: usize, f: F) {
        f(&mut self.get(id))
    }
}

#[cfg(test)]
mod tests {
    fn hello_world<V: core::fmt::Display>(v: V) -> String {
        format!("hello, {}", v)
    }

    macro_rules! hello {
        () => {
            eprintln!("{}", hello_world("macro"));
        };
    }

    fn hello() {
        hello!();
        eprintln!("{}", hello_world(String::from("string")));
    }

    #[test]
    fn hello_test() {
        hello();
        hello!();
    }
}
