mod build_support;
use crate::build_support::skia::FinalBuildConfiguration;
use build_support::{azure, binaries, cargo, git, skia, utils};
use std::io::Cursor;
use std::path::Path;
use std::{fs, io};

/// Environment variables used by this build script.
mod env {
    use crate::build_support::cargo;

    /// Returns true if the download should be forced. This can be used to test prebuilt binaries
    /// from within a repository build. If this environment variable is not set, binaries
    /// are downloaded only in crate builds.
    pub fn force_skia_binaries_download() -> bool {
        cargo::env_var("FORCE_SKIA_BINARIES_DOWNLOAD").is_some()
    }

    /// Force to build skia, even if there is a binary available.
    pub fn force_skia_build() -> bool {
        cargo::env_var("FORCE_SKIA_BUILD").is_some()
    }
}

const SRC_BINDINGS_RS: &str = "src/bindings.rs";
const SKIA_LICENSE: &str = "skia/LICENSE";

fn main() {
    let build_config = skia::BuildConfiguration::default();
    let binaries_config = skia::BinariesConfiguration::from_cargo_env(&build_config);

    //
    // is the download of prebuilt binaries possible?
    //

    let build_skia = env::force_skia_build() || {
        if let Some((tag, key)) = should_try_download_binaries(&binaries_config) {
            println!(
                "TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: {}/{}",
                tag, key
            );
            let url = binaries::download_url(tag, key);
            println!("  FROM: {}", url);
            if let Err(e) = download_and_install(url, &binaries_config.output_directory) {
                println!("DOWNLOAD AND INSTALL FAILED: {}", e);
                true
            } else {
                println!("DOWNLOAD AND INSTALL SUCCEEDED");
                false
            }
        } else {
            true
        }
    };

    //
    // full build?
    //

    if build_skia {
        println!("STARTING A FULL BUILD");
        let final_configuration = FinalBuildConfiguration::from_build_configuration(&build_config);
        skia::build(&final_configuration, &binaries_config);
    }

    binaries_config.commit_to_cargo();

    //
    // publish binaries?
    //

    if let Some(staging_directory) = binaries::should_export() {
        println!(
            "DETECTED AZURE, exporting binaries to {}",
            staging_directory.to_str().unwrap()
        );

        println!("EXPORTING BINARIES");
        let source_files = &[
            (SRC_BINDINGS_RS, "bindings.rs"),
            (SKIA_LICENSE, "LICENSE_SKIA"),
        ];
        binaries::export(&binaries_config, source_files, &staging_directory)
            .expect("EXPORTING BINARIES FAILED")
    }
}

/// If the binaries should be downloaded, return the tag and the key.
fn should_try_download_binaries(config: &skia::BinariesConfiguration) -> Option<(String, String)> {
    let tag = cargo::package_version();

    // for testing:
    if env::force_skia_binaries_download() {
        // retrieve the hash from the repository above us.
        let half_hash = git::half_hash()?;
        return Some((tag, config.key(&half_hash)));
    }

    // are we building inside a crate?
    if let Ok(ref full_hash) = cargo::crate_repository_hash() {
        let half_hash = git::trim_hash(full_hash);
        return Some((tag, config.key(&half_hash)));
    }

    None
}

fn download_and_install(url: impl AsRef<str>, output_directory: &Path) -> io::Result<()> {
    let archive = utils::download(url)?;
    println!(
        "UNPACKING ARCHIVE INTO: {}",
        output_directory.to_str().unwrap()
    );
    binaries::unpack(Cursor::new(archive), output_directory)?;
    // TODO: verify key?
    println!("INSTALLING BINDINGS");
    fs::copy(output_directory.join("bindings.rs"), SRC_BINDINGS_RS)?;

    Ok(())
}
