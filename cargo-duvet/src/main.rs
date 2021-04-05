use anyhow::Result;
use duvet::{
    attribute,
    coverage::{self, llvm},
    db::Db,
    notification, types,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{sync::Arc, thread};

attribute!(const TEST_ID: u32);

mod manifest;
mod process;
mod project;
mod test;

fn main() -> Result<()> {
    let db = Db::new()?;
    let project = project::Builder::default().build()?;
    project.install_llvm_tools()?;
    let tests = test::list::List::from_project(&project)?;

    let style = ProgressStyle::default_bar()
        .template("{prefix:>12.green.bold} [{bar:57}] {pos}/{len} {msg}")
        .progress_chars("=> ");

    let m = MultiProgress::new();

    m.set_draw_target(indicatif::ProgressDrawTarget::stdout());

    let pb_test = m.add(ProgressBar::new(tests.as_slice().len() as _));
    pb_test.set_style(style.clone());
    pb_test.set_prefix("Testing");

    let pb_analyze = m.add(ProgressBar::new(5));
    pb_analyze.set_style(style);
    pb_analyze.set_prefix("Analyzing");

    let llvm_code = db.entities().create()?;
    db.entities()
        .set_attribute(llvm_code, duvet::types::CODE, ())?;

    // mark significant lines
    for data in tests.profdata(&project) {
        let export: llvm::Export = data?;
        export.visit(
            &db,
            &llvm::FnVisitor(|file, bytes, _execution_count| {
                db.regions().insert(file, bytes, llvm_code)?;
                Ok(())
            }),
        )?;
    }

    let batch = thread::spawn(move || -> Result<Db> {
        tests.run(|test| {
            pb_test.set_message(test.name());

            let test_entity = db.entities().create()?;
            db.entities()
                .set_attribute(test_entity, TEST_ID, test.id() as u32)?;
            db.entities()
                .set_attribute(test_entity, duvet::types::TEST_REGION, ())?;

            let export: llvm::Export = test.run(&project)?;

            export.visit(
                &db,
                &llvm::FnVisitor(|file, bytes, execution_count| {
                    if execution_count > 0 {
                        // TODO save execution count
                        db.regions().insert(file, bytes, test_entity)?;
                    }
                    Ok(())
                }),
            )?;

            pb_test.inc(1);

            Ok(())
        })?;

        pb_test.finish_with_message("done");

        /*
        pb_analyze.set_message("Rust source");
        duvet::rust_src::RustSrc::default().annotate(&db)?;
        pb_analyze.inc(1);
        */

        pb_analyze.set_message("Highlighting");
        pb_analyze.set_length(pb_analyze.length() + db.fs().len() as u64);
        for _ in duvet::highlight::highlight_all(&db) {
            pb_analyze.inc(1);
        }

        pb_analyze.set_message("Calculating regions");
        db.finish_regions()?;
        pb_analyze.inc(1);

        let mut handler = report::Handler::new(&db, &tests);

        pb_analyze.set_message("Calculating notifications");
        duvet::coverage::notify(&db, types::CODE, types::TEST_REGION, &mut handler)?;
        pb_analyze.inc(1);

        pb_analyze.set_message("Finishing notifications");
        db.finish_notifications()?;
        pb_analyze.inc(1);

        pb_analyze.set_message("Generating reports");
        let html = duvet::html::Config::default();
        db.fs().par_for_each(|file| html.file(&db, file))?;

        pb_analyze.finish_with_message("done");

        Ok(db)
    });

    m.join()?;

    drop(batch);

    Ok(())
}

mod report {
    use super::*;
    use core::ops::Range;
    use duvet::schema::{EntityId, FileId};
    use test::list::List;

    pub struct Handler<'a> {
        db: &'a Db,
        tests: &'a List,
        failure: Arc<dyn notification::Notification>,
        failure_id: Option<notification::Id>,
    }

    impl<'a> Handler<'a> {
        pub fn new(db: &'a Db, tests: &'a List) -> Self {
            let failure = Arc::new(notification::Simple {
                title: "Missing test coverage".to_string(),
                ..Default::default()
            });
            Self {
                db,
                tests,
                failure,
                failure_id: None,
            }
        }
    }

    impl<'a> coverage::Handler for Handler<'a> {
        fn on_region_success(
            &mut self,
            file: FileId,
            bytes: Range<u32>,
            _entity: EntityId,
            references: &[EntityId],
        ) -> Result<()> {
            // TODO list all of the test references
            let notification: Arc<dyn notification::Notification> =
                Arc::new(notification::Simple {
                    title: "Has test coverage".to_string(),
                    ..Default::default()
                });

            let id = self
                .db
                .notifications()
                .create(notification::Level::Success, notification);

            self.db.notifications().notify(file, bytes, id)?;

            Ok(())
        }

        fn on_region_failure(
            &mut self,
            file: FileId,
            bytes: Range<u32>,
            _entity: EntityId,
        ) -> Result<()> {
            let id = if let Some(id) = self.failure_id {
                id
            } else {
                let id = self
                    .db
                    .notifications()
                    .create(notification::Level::Error, self.failure.clone());
                self.failure_id = Some(id);
                id
            };

            self.db.notifications().notify(file, bytes, id)?;

            Ok(())
        }
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
