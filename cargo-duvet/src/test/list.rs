use crate::{
    process::{exec, Command, StatusAsResult},
    project::Project,
};
use anyhow::{anyhow, Result};
use rayon::prelude::*;

#[derive(Debug)]
pub struct List {
    tests: Vec<Test>,
    binaries: Vec<String>,
}

impl List {
    pub fn from_project(project: &Project) -> Result<Self> {
        let build_profraw = project.profraw_file(&"build");
        std::fs::create_dir_all(build_profraw.parent().unwrap())?;

        // build everything first
        let mut build = project.cargo("test");
        build
            .arg("--all")
            .arg("--no-run")
            .env("LLVM_PROFILE_FILE", &build_profraw);
        exec(build)?;

        let mut list = project.cargo("test");

        list.arg("--all")
            .arg("--")
            .arg("--list")
            .arg("--format")
            .arg("terse")
            .env("LLVM_PROFILE_FILE", &build_profraw);

        let result = list
            .output()
            .expect("list should always work")
            .status_as_result()
            .expect("list should always work");

        let binaries: Vec<_> = find_binary_paths(&result.stderr)
            .map(String::from)
            .collect();

        let mut tests = binaries
            .par_iter()
            .enumerate()
            .flat_map(|(id, binary)| {
                let list_profraw = project.profraw_file(&format!("list_{}", id));

                let mut list = Command::new(binary);
                list.arg("--list")
                    .arg("--format")
                    .arg("terse")
                    .env("LLVM_PROFILE_FILE", &list_profraw);

                let result = list
                    .output()
                    .expect("list should always work")
                    .status_as_result()
                    .expect("list should always work");

                let stdout = core::str::from_utf8(&result.stdout).expect("invalid list output");

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
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for (id, test) in tests.iter_mut().enumerate() {
            test.id = id;
        }

        Ok(Self { tests, binaries })
    }

    pub fn as_slice(&self) -> &[Test] {
        &self.tests
    }

    pub fn profdata<'a, T: serde::de::DeserializeOwned>(
        &'a self,
        project: &'a Project,
    ) -> impl Iterator<Item = Result<T>> + 'a {
        self.binaries
            .iter()
            .enumerate()
            .map(move |(id, bin)| project.profdata(bin, &format!("list_{}", id)))
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
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn binary(&self) -> &str {
        &self.binary
    }

    pub fn run<T: serde::de::DeserializeOwned>(&self, project: &Project) -> Result<T> {
        let mut test = Command::new(&self.binary);
        let profraw = project.profraw_file(&self.id);

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

        project.profdata(&self.binary, &self.id)
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

fn find_binary_paths(stderr: &[u8]) -> impl Iterator<Item = &str> {
    core::str::from_utf8(&stderr)
        .ok()
        .map(|s| {
            s.split('\n').filter_map(|line| {
                let line = line.trim();
                let mut line = line.split("Running ");
                line.next()?;
                let v = line.next()?;
                if v.ends_with(')') {
                    let v = v.split('(').last()?;
                    Some(&v[..v.len() - 1])
                } else {
                    Some(v)
                }
            })
        })
        .into_iter()
        .flatten()
}
