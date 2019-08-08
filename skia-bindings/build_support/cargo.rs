//! Support function for communicating with cargo's variables and outputs.

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::{env, fmt, fs, io};

pub fn output_directory() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

pub fn add_dependent_path(path: impl AsRef<Path>) {
    println!("cargo:rerun-if-changed={}", path.as_ref().to_str().unwrap());
}

pub fn add_link_libs(libs: &[impl AsRef<str>]) {
    libs.into_iter().for_each(|s| add_link_lib(s.as_ref()))
}

pub fn add_link_lib(lib: impl AsRef<str>) {
    println!("cargo:rustc-link-lib={}", lib.as_ref());
}

#[derive(Clone, Debug)]
pub struct Target {
    pub architecture: String,
    pub vendor: String,
    pub system: String,
    pub abi: Option<String>,
}

impl Target {
    pub fn as_strs(&self) -> (&str, &str, &str, Option<&str>) {
        (
            self.architecture.as_str(),
            self.vendor.as_str(),
            self.system.as_str(),
            self.abi.as_ref().map(|s| s.as_str()),
        )
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            &self.architecture, &self.vendor, &self.system
        )?;

        if let Some(ref abi) = self.abi {
            write!(f, "-{}", abi)
        } else {
            Result::Ok(())
        }
    }
}

pub fn target() -> Target {
    let target_str = env::var("TARGET").unwrap();
    parse_target(target_str)
}

pub fn host() -> Target {
    let host_str = env::var("HOST").unwrap();
    println!("HOST: {}", host_str);
    parse_target(host_str)
}

fn parse_target(target_str: impl AsRef<str>) -> Target {
    let target_str = target_str.as_ref();
    let target: Vec<String> = target_str.split("-").map(|s| s.into()).collect();
    if target.len() < 3 {
        panic!("Failed to parse TARGET {}", target_str);
    }

    let abi = if target.len() > 3 {
        Some(target[3].clone())
    } else {
        None
    };

    Target {
        architecture: target[0].clone(),
        vendor: target[1].clone(),
        system: target[2].clone(),
        abi,
    }
}

// We can not assume that the build profile of the build.rs script reflects the build
// profile that the target needs.
#[allow(dead_code)]
pub fn build_release() -> bool {
    match env::var("PROFILE").unwrap().as_str() {
        "release" => true,
        "debug" => false,
        _ => panic!("PROFILE '{}' is not supported by this build script",),
    }
}

/// Are we inside the crate?
pub fn is_crate() -> bool {
    package_repository_hash().is_ok()
}

// If we are builing from within a packaged crate, return the full commit hash
// of the original repository we were packaged from.
pub fn package_repository_hash() -> io::Result<String> {
    let vcs_info = fs::read_to_string(".cargo_vcs_info.json")?;
    let value: serde_json::Value = serde_json::from_str(&vcs_info)?;
    let git = value.get("git").expect("failed to get 'git' property");
    let sha1 = git.get("sha1").expect("failed to get 'sha1' property");
    Ok(sha1.as_str().unwrap().into())
}

pub fn package_version() -> String {
    env::var("CARGO_PKG_VERSION").unwrap().as_str().into()
}

/// Parses Cargo.toml and returns the metadadata specifed in the
/// [package.metadata] section.
pub fn get_metadata() -> Vec<(String, String)> {
    use toml::{de, value};

    let cargo_toml = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("missing environment variable CARGO_MANIFEST_DIR"),
    )
    .join("Cargo.toml");
    let str = fs::read_to_string(cargo_toml).expect("Failed to read Cargo.toml");
    let root: value::Table =
        de::from_str::<value::Table>(&str).expect("Failed to parse Cargo.toml");
    let manifest_table: &value::Table = root
        .get("package")
        .expect("section [package] missing")
        .get("metadata")
        .expect("section [package.metadata] missing")
        .as_table()
        .unwrap();

    manifest_table
        .iter()
        .map(|(a, b)| (a.clone(), b.to_string()))
        .collect()
}
