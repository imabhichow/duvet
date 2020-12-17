use crate::{
    manifest::Manifest,
    process::{Command, StatusAsResult},
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct Builder {
    pub toolchain: String,
    pub manifest_path: Option<PathBuf>,
    pub release: bool,
    pub target: String,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            toolchain: "nightly".to_owned(),
            manifest_path: None,
            release: false,
            target: env!("DEFAULT_TARGET").to_owned(),
        }
    }
}

impl Builder {
    pub fn build(self) -> Result<Project> {
        let llvm_dir = self.llvm_dir()?;
        let manifest = self.manifest()?;
        let cargo_toolchain = self.toolchain();

        let Self {
            toolchain,
            manifest_path,
            release,
            target,
        } = self;

        Ok(Project {
            llvm_dir,
            manifest,
            manifest_path,
            cargo_toolchain,
            toolchain,
            release,
            target,
        })
    }

    fn manifest(&self) -> Result<Manifest> {
        let mut cmd = Command::new("cargo");

        cmd.arg(&self.toolchain())
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--no-deps");

        self.with_args(&mut cmd);

        let result = cmd.output()?.status_as_result()?;
        let metadata = serde_json::from_slice(&result.stdout)?;
        Ok(metadata)
    }

    fn llvm_dir(&self) -> Result<PathBuf> {
        let mut cmd = Command::new("rustup");
        cmd.arg(&self.toolchain()).arg("which").arg("rustc");
        let result = cmd.output()?.status_as_result()?;
        let rustc = core::str::from_utf8(&result.stdout)
            .expect("invalid rustc path")
            .trim();
        let mut path = PathBuf::from(rustc);
        path.pop(); // rustc
        path.pop(); // bin
        path.push("lib");
        path.push("rustlib");
        path.push(&self.target);
        path.push("bin");

        Ok(path)
    }

    fn with_args(&self, cmd: &mut Command) {
        if let Some(path) = self.manifest_path.as_ref() {
            cmd.arg("--manifest-path").arg(path);
        }

        if self.release {
            cmd.arg("--release");
        }
    }

    fn toolchain(&self) -> String {
        format!("+{}", self.toolchain)
    }
}

#[derive(Debug)]
pub struct Project {
    pub toolchain: String,
    pub cargo_toolchain: String,
    pub manifest_path: Option<PathBuf>,
    pub manifest: Manifest,
    pub release: bool,
    pub llvm_dir: PathBuf,
    pub target: String,
}

impl Project {
    pub fn cargo(&self, c: &str) -> Command {
        let mut cmd = Command::new("cargo");

        cmd.arg(&self.cargo_toolchain)
            .arg(c)
            .arg("--target-dir")
            .arg("target/coverage")
            .env("RUSTFLAGS", "-Zinstrument-coverage");

        if let Some(path) = self.manifest_path.as_ref() {
            cmd.arg("--manifest-path").arg(path);
        }

        if self.release {
            cmd.arg("--release");
        }

        cmd
    }

    pub fn llvm_bin(&self, name: &str) -> Command {
        let bin = self.llvm_dir.join(Path::new(name));
        Command::new(bin)
    }
}
