use crate::{
    process::{exec, Command, StatusAsResult},
    project::Project,
};
use anyhow::{anyhow, Result};
use rayon::prelude::*;

#[derive(Debug)]
pub struct List {
    tests: Vec<Test>,
}

impl List {
    pub fn from_project(project: &Project) -> Result<Self> {
        // build everything first
        let mut build = project.cargo("test");
        build
            .arg("--all")
            .arg("--no-run")
            .env("LLVM_PROFILE_FILE", "target/coverage/_build.profdata");
        exec(build)?;

        let targets: Vec<_> = project
            .manifest
            .packages
            .iter()
            .flat_map(|package| {
                let name = &package.name;
                package.targets.iter().map(move |target| (name, target))
            })
            .collect();

        let mut tests = targets
            .par_iter()
            .flat_map(|(package_name, target)| {
                let mut tests = vec![];

                for kind in &target.kind {
                    let mut list = project.cargo("test");
                    list.arg("--package").arg(package_name);

                    if kind_args(kind, &target.name, &mut list).is_none() {
                        continue;
                    }

                    list.arg("--")
                        .arg("--list")
                        .env("LLVM_PROFILE_FILE", "target/coverage/_list.profdata");

                    let result = list
                        .output()
                        .expect("list should always work")
                        .status_as_result()
                        .expect("list should always work");

                    let binary = find_binary_path(&result.stderr).expect("missing binary path");

                    let stdout = core::str::from_utf8(&result.stdout).expect("invalid list output");

                    tests.extend(
                        stdout
                            .split('\n')
                            .filter(|line| !line.is_empty())
                            .filter_map(|line| {
                                let mut line = line.split(": ");
                                let name = line.next()?;
                                let ty = line.next()?;
                                Some([name, ty])
                            })
                            .map(|[name, _ty]| Test {
                                id: 0, // this will be initialized later
                                binary: binary.to_string(),
                                name: name.to_string(),
                            }),
                    );
                }

                tests
            })
            .collect::<Vec<_>>();

        for (id, test) in tests.iter_mut().enumerate() {
            test.id = id;
        }

        Ok(Self { tests })
    }

    pub fn run<F>(&self, run: F) -> Result<()>
    where
        F: Send + Sync + Fn(&Test) -> Result<()>,
    {
        let results: Vec<_> = self
            .tests
            .par_iter()
            .filter_map(move |test| run(test).err())
            .collect();

        if results.is_empty() {
            Ok(())
        } else {
            let mut err = anyhow!("{} tests failed", results.len());
            for result in results {
                err = err.context(result);
            }
            Err(err)
        }
    }
}

#[derive(Debug)]
pub struct Test {
    id: usize,
    name: String,
    binary: String,
}

impl Test {
    pub fn run<T: serde::de::DeserializeOwned>(&self, project: &Project) -> Result<T> {
        let mut test = Command::new(&self.binary);
        let profraw = format!("target/coverage/{}.profraw", self.id);
        let profdata = format!("target/coverage/{}.profdata", self.id);

        test.arg("--exact")
            .arg(&self.name)
            .env("LLVM_PROFILE_FILE", &profraw);

        let result = test.output()?;
        if result.status.code().map_or(false, |code| code != 0) {
            return Err(anyhow!(
                "{}",
                core::str::from_utf8(&result.stderr).unwrap_or("Test failed")
            ));
        }

        let mut merge = project.llvm_bin("llvm-profdata");
        merge
            .arg("merge")
            .arg("-sparse")
            .arg(&profraw)
            .arg("-o")
            .arg(&profdata);

        exec(merge)?;

        // llvm-cov is not included with rustup
        let mut export = Command::new("llvm-cov");
        export
            .arg("export")
            .arg(&self.binary)
            .arg("-instr-profile")
            .arg(&profdata)
            .arg("-format=text");

        let result = export.output()?.status_as_result()?;
        let coverage = serde_json::from_slice(&result.stdout)?;
        Ok(coverage)
    }
}

fn kind_args(kind: &str, name: &str, cmd: &mut Command) -> Option<()> {
    match kind {
        "lib" => cmd.arg("--lib"),
        "test" => cmd.arg("--test").arg(name),
        "bin" => cmd.arg("--bin").arg(name),
        "example" => cmd.arg("--example").arg(name),
        "bench" => cmd.arg("--bench").arg(name),
        // ignore kinds that we don't understand
        _ => return None,
    };

    Some(())
}

fn find_binary_path(stderr: &[u8]) -> Option<&str> {
    core::str::from_utf8(&stderr)
        .ok()?
        .split('\n')
        .find_map(|line| {
            let line = line.trim();
            let mut line = line.split("Running ");
            line.next()?;
            line.next()
        })
}
