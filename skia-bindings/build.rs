mod build_support;
use build_support::{binaries, cargo, git, skia, utils};
use std::io;
use std::io::Cursor;
use std::path::Path;

/// Environment variables used by this build script.
mod env {
    use crate::build_support::cargo;
    use std::path::PathBuf;

    /// Returns `true` if the download of prebuilt binaries should be forced.
    ///
    /// This can be used to test and downlaod prebuilt binaries from within a repository build.
    /// If this environment variable is not set, binaries are downloaded from crate builds only.
    pub fn force_skia_binaries_download() -> bool {
        cargo::env_var("FORCE_SKIA_BINARIES_DOWNLOAD").is_some()
    }

    /// The URL template to download the Skia binaries from.
    ///
    /// `{tag}` will be replaced by the Tag (usually the released skia-binding's crate's version).
    /// `{key}` will be replaced by the Key (a combination of the repository hash, target, and features).
    ///
    /// `file://` URLs are supported for local testing.
    pub fn skia_binaries_url() -> Option<String> {
        cargo::env_var("SKIA_BINARIES_URL")
    }

    /// The default URL template to download the binaries from.
    pub fn skia_binaries_url_default() -> String {
        "https://github.com/rust-skia/skia-binaries/releases/download/{tag}/skia-binaries-{key}.tar.gz".into()
    }

    /// Force to build Skia, even if there is a binary available.
    pub fn force_skia_build() -> bool {
        cargo::env_var("FORCE_SKIA_BUILD").is_some()
    }

    /// A boolean specifying whether to build Skia's dependencies or not. If not, the system's
    /// provided libraries are used.
    pub fn use_system_libraries() -> bool {
        cargo::env_var("SKIA_USE_SYSTEM_LIBRARIES").is_some()
    }

    /// The full path of the ninja command to run.
    pub fn ninja_command() -> Option<PathBuf> {
        cargo::env_var("SKIA_NINJA_COMMAND").map(PathBuf::from)
    }

    /// The full path of the gn command to run.
    pub fn gn_command() -> Option<PathBuf> {
        cargo::env_var("SKIA_GN_COMMAND").map(PathBuf::from)
    }

    /// The path to the Skia source directory.
    pub fn source_dir() -> Option<PathBuf> {
        cargo::env_var("SKIA_SOURCE_DIR").map(PathBuf::from)
    }
}

const SKIA_LICENSE: &str = "skia/LICENSE";

fn main() {
    // since 0.25.0
    if cfg!(feature = "svg") {
        cargo::warning("The feature 'svg' has been removed. SVG and XML support is available in all build configurations.");
    }
    // since 0.25.0
    if cfg!(feature = "shaper") {
        cargo::warning("The feature 'shaper' has been removed. To use the SkShaper bindings, enable the feature 'textlayout'.");
    }

    let build_config = skia::BuildConfiguration::default();
    let binaries_config = skia::BinariesConfiguration::from_cargo_env(&build_config);

    //
    // skip attempting to download?
    //
    if let Some(source_dir) = env::source_dir() {
        println!("STARTING OFFLINE BUILD");

        let final_configuration = skia::FinalBuildConfiguration::from_build_configuration(
            &build_config,
            env::use_system_libraries(),
            &source_dir,
        );

        skia::build(
            &final_configuration,
            &binaries_config,
            env::ninja_command(),
            env::gn_command(),
            true,
        );
    } else {
        //
        // is the download of prebuilt binaries possible?
        //

        let build_skia = env::force_skia_build() || {
            let force_download = env::force_skia_binaries_download();
            if let Some((tag, key)) = should_try_download_binaries(&binaries_config, force_download)
            {
                println!(
                    "TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: {}/{}",
                    tag, key
                );
                let url = binaries::download_url(
                    env::skia_binaries_url().unwrap_or_else(env::skia_binaries_url_default),
                    tag,
                    key,
                );
                println!("  FROM: {}", url);
                if let Err(e) = download_and_install(url, &binaries_config.output_directory) {
                    println!("DOWNLOAD AND INSTALL FAILED: {}", e);
                    if force_download {
                        panic!("Downloading of binaries was forced but failed.")
                    }
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
            let final_configuration = skia::FinalBuildConfiguration::from_build_configuration(
                &build_config,
                env::use_system_libraries(),
                &std::env::current_dir().unwrap().join("skia"),
            );
            skia::build(
                &final_configuration,
                &binaries_config,
                env::ninja_command(),
                env::gn_command(),
                false,
            );
        }
    };

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
        let source_files = &[(SKIA_LICENSE, "LICENSE_SKIA")];
        binaries::export(&binaries_config, source_files, &staging_directory)
            .expect("EXPORTING BINARIES FAILED")
    }
}

/// If the binaries should be downloaded, return the tag and the key.
fn should_try_download_binaries(
    config: &skia::BinariesConfiguration,
    force: bool,
) -> Option<(String, String)> {
    let tag = cargo::package_version();

    // for testing:
    if force {
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

    Ok(())
}
