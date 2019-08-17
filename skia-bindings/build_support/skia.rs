//! Full build support for the Skia library, SkiaBindings library and bindings.rs file.

use crate::build_support::{android, cargo, clang, ios, vs};
use cc::Build;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

mod lib {
    pub const SKIA: &str = "skia";
    pub const SKIA_BINDINGS: &str = "skia-bindings";
    pub const SKSHAPER: &str = "skshaper";
}

mod feature {
    pub const VULKAN: &str = "vulkan";
    pub const SVG: &str = "svg";
    pub const SHAPER: &str = "shaper";
}

/// The defaults for the Skia build configuration.
impl Default for BuildConfiguration {
    fn default() -> Self {
        // m74: if we don't build the particles or the skottie library on macOS, the build fails with
        // for example:
        // [763/867] link libparticles.a
        // FAILED: libparticles.a
        let all_skia_libs = {
            match cargo::target().as_strs() {
                (_, "apple", "darwin", _) => true,
                (_, "apple", "ios", _) => true,
                _ => false,
            }
        };

        BuildConfiguration {
            on_windows: cargo::host().is_windows(),
            // Note that currently, we don't support debug Skia builds,
            // because they are hard to configure and pull in a lot of testing related modules.
            skia_release: true,
            feature_vulkan: cfg!(feature = "vulkan"),
            feature_svg: cfg!(feature = "svg"),
            feature_shaper: cfg!(feature = "shaper"),
            feature_animation: false,
            feature_dng: false,
            feature_particles: false,
            all_skia_libs,
            definitions: Vec::new(),
        }
    }
}

/// The build configuration for Skia.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildConfiguration {
    /// Do we build _on_ a Windows OS?
    on_windows: bool,

    /// Build Skia in a release configuration?
    skia_release: bool,

    /// Build with Vulkan support?
    feature_vulkan: bool,

    /// Build with SVG support?
    feature_svg: bool,

    /// Builds the skshaper module, compiles harfbuzz & icu support.
    feature_shaper: bool,

    /// Build with animation support (yet unsupported, no wrappers).
    feature_animation: bool,

    /// Support DNG file format (currently unsupported because of build errors).
    feature_dng: bool,

    /// Build the particles module (unsupported, no wrappers).
    feature_particles: bool,

    /// As of M74, There is a bug in the Skia macOS build
    /// that requires all libraries to be built, otherwise the build will fail.
    all_skia_libs: bool,

    /// Additional preprocessor definitions that will override predefined ones.
    definitions: Definitions,
}

/// This is the final, low level build configuration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FinalBuildConfiguration {
    /// The name value pairs passed as arguments to gn.
    pub gn_args: Vec<(String, String)>,

    /// The additional definitions (cloned from the definitions of
    /// the BuildConfiguration).
    pub definitions: Definitions,

    /// The binding source files to compile.
    pub binding_sources: Vec<PathBuf>,
}

impl FinalBuildConfiguration {
    pub fn from_build_configuration(build: &BuildConfiguration) -> FinalBuildConfiguration {
        let gn_args = {
            fn yes() -> String {
                "true".into()
            }
            fn no() -> String {
                "false".into()
            }

            fn quote(s: &str) -> String {
                format!("\"{}\"", s)
            }

            let mut args: Vec<(&str, String)> = vec![
                (
                    "is_official_build",
                    if build.skia_release { yes() } else { no() },
                ),
                ("skia_use_system_libjpeg_turbo", no()),
                ("skia_use_system_libpng", no()),
                ("skia_use_libwebp", no()),
                ("skia_use_system_zlib", no()),
                (
                    "skia_enable_skottie",
                    if build.feature_animation || build.all_skia_libs {
                        yes()
                    } else {
                        no()
                    },
                ),
                ("skia_use_xps", no()),
                (
                    "skia_use_dng_sdk",
                    if build.feature_dng { yes() } else { no() },
                ),
                (
                    "skia_enable_particles",
                    if build.feature_particles || build.all_skia_libs {
                        yes()
                    } else {
                        no()
                    },
                ),
                ("cc", quote("clang")),
                ("cxx", quote("clang++")),
            ];

            // further flags that limit the components of Skia debug builds.
            if !build.skia_release {
                args.push(("skia_enable_atlas_text", no()));
                args.push(("skia_enable_spirv_validation", no()));
                args.push(("skia_enable_tools", no()));
                args.push(("skia_enable_vulkan_debug_layers", no()));
                args.push(("skia_use_libheif", no()));
                args.push(("skia_use_lua", no()));
            }

            if build.feature_shaper {
                args.push(("skia_enable_skshaper", yes()));
                args.push(("skia_use_icu", yes()));
                args.push(("skia_use_system_icu", no()));
                args.push(("skia_use_harfbuzz", yes()));
                args.push(("skia_use_system_harfbuzz", no()));
                args.push(("skia_use_sfntly", no()));
            } else {
                args.push(("skia_use_icu", no()));
            }

            if build.feature_vulkan {
                args.push(("skia_use_vulkan", yes()));
                args.push(("skia_enable_spirv_validation", no()));
            }

            let mut flags: Vec<&str> = vec![];
            let mut use_expat = build.feature_svg;

            // target specific gn args.
            let target = cargo::target();
            match target.as_strs() {
                (_, _, "windows", Some("msvc")) if build.on_windows => {
                    if let Some(win_vc) = vs::resolve_win_vc() {
                        args.push(("win_vc", quote(win_vc.to_str().unwrap())))
                    }
                    // Rust's msvc toolchain links to msvcrt.dll by
                    // default for release and _debug_ builds.
                    flags.push("/MD");
                    // Tell Skia's build system where LLVM is supposed to be located.
                    // TODO: this should be checked as a prerequisite.
                    args.push(("clang_win", quote("C:/Program Files/LLVM")));
                }
                (arch, "linux", "android", _) => {
                    args.push(("ndk", quote(&android::ndk())));
                    args.push(("target_cpu", quote(clang::target_arch(arch))));
                    args.push(("skia_use_system_freetype2", no()));
                    args.push(("skia_enable_fontmgr_android", yes()));
                    // Enabling fontmgr_android implicitly enables expat.
                    // We make this explicit to avoid relying on an expat installed
                    // in the system.
                    use_expat = true;
                }
                (arch, "apple", "ios", _) => {
                    args.push(("target_os", quote("ios")));
                    args.push(("target_cpu", quote(clang::target_arch(arch))));
                }
                _ => {}
            }

            if use_expat {
                args.push(("skia_use_expat", yes()));
                args.push(("skia_use_system_expat", no()));
            } else {
                args.push(("skia_use_expat", no()));
            }

            if !flags.is_empty() {
                let flags: String = {
                    let v: Vec<String> = flags.into_iter().map(quote).collect();
                    v.join(",")
                };
                args.push(("extra_cflags", format!("[{}]", flags)));
            }

            args.into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        };

        let binding_sources = {
            let mut sources: Vec<PathBuf> = Vec::new();
            sources.push("src/bindings.cpp".into());
            if build.feature_shaper {
                sources.push("src/shaper.cpp".into())
            };
            sources
        };

        FinalBuildConfiguration {
            gn_args,
            definitions: build.definitions.clone(),
            binding_sources,
        }
    }
}

/// The resulting binaries configuration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BinariesConfiguration {
    /// The features we built with.
    pub features: Vec<String>,

    /// The output directory of the libraries we build and we need to inform cargo about.
    pub output_directory: PathBuf,

    /// The TARGET specific link libraries we need to inform cargo about.
    pub link_libraries: Vec<String>,

    /// The static Skia libraries skia-bindings provides and dependent projects need to link with.
    pub built_libraries: Vec<String>,

    /// Additional files relative to the output_directory
    /// that are needed to build dependent projects.
    pub additional_files: Vec<PathBuf>,
}

const SKIA_OUTPUT_DIR: &str = "skia";
const ICUDTL_DAT: &str = "icudtl.dat";

impl BinariesConfiguration {
    /// Build a binaries configuration based on the current environment cargo
    /// supplies us with and a Skia build configuration.
    pub fn from_cargo_env(build: &BuildConfiguration) -> Self {
        let mut built_libraries = Vec::new();
        let mut additional_files = Vec::new();
        let mut features = Vec::new();
        if build.feature_vulkan {
            features.push(feature::VULKAN);
        }
        if build.feature_svg {
            features.push(feature::SVG);
        }
        if build.feature_shaper {
            features.push(feature::SHAPER);
            additional_files.push(ICUDTL_DAT.into());
            built_libraries.push(lib::SKSHAPER.into());
        }

        let mut link_libraries = Vec::new();

        match cargo::target().as_strs() {
            (_, "unknown", "linux", Some("gnu")) => {
                link_libraries.extend(vec!["stdc++", "bz2", "GL", "fontconfig", "freetype"]);
            }
            (_, "apple", "darwin", _) => {
                link_libraries.extend(vec![
                    "c++",
                    "framework=OpenGL",
                    "framework=ApplicationServices",
                ]);
            }
            (_, _, "windows", Some("msvc")) => {
                link_libraries.extend(vec![
                    "usp10", "ole32", "user32", "gdi32", "fontsub", "opengl32",
                ]);
            }
            (_, "linux", "android", _) => {
                link_libraries.extend(vec!["log", "android", "EGL", "GLESv2"]);
            }
            (_, "apple", "ios", _) => {
                link_libraries.extend(vec![
                    "c++",
                    "framework=MobileCoreServices",
                    "framework=CoreFoundation",
                    "framework=CoreGraphics",
                    "framework=CoreText",
                    "framework=ImageIO",
                    "framework=UIKit",
                ]);
            }
            _ => panic!("unsupported target: {:?}", cargo::target()),
        };

        let output_directory = cargo::output_directory()
            .join(SKIA_OUTPUT_DIR)
            .to_str()
            .unwrap()
            .into();

        built_libraries.push(lib::SKIA.into());
        built_libraries.push(lib::SKIA_BINDINGS.into());

        BinariesConfiguration {
            features: features.iter().map(|f| f.to_string()).collect(),
            output_directory,
            link_libraries: link_libraries.iter().map(|lib| lib.to_string()).collect(),
            built_libraries,
            additional_files,
        }
    }

    /// Inform cargo that the output files of the given configuration are available and
    /// can be used as dependencies.
    pub fn commit_to_cargo(&self) {
        cargo::add_link_libs(&self.link_libraries);
        println!(
            "cargo:rustc-link-search={}",
            self.output_directory.to_str().unwrap()
        );

        for lib in &self.built_libraries {
            cargo::add_link_lib(format!("static={}", lib));
        }
    }
}

/// The full build of Skia, SkiaBindings, and the generation of bindings.rs.
pub fn build(build: &FinalBuildConfiguration, config: &BinariesConfiguration) {
    prerequisites::resolve_dependencies();

    let python2 = &prerequisites::locate_python2_cmd();
    println!("Python 2 found: {:?}", python2);

    assert!(
        Command::new(python2)
            .arg("skia/tools/git-sync-deps")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .unwrap()
            .success(),
        "`skia/tools/git-sync-deps` failed"
    );

    let gn_args = build
        .gn_args
        .iter()
        .map(|(name, value)| name.clone() + "=" + value)
        .collect::<Vec<String>>()
        .join(" ");

    let on_windows = cfg!(windows);

    let gn_command = if on_windows { "skia/bin/gn" } else { "bin/gn" };

    let output_directory_str = config.output_directory.to_str().unwrap();

    let output = Command::new(gn_command)
        .args(&[
            "gen",
            output_directory_str,
            &format!("--script-executable={}", python2),
            &format!("--args={}", gn_args),
        ])
        .envs(env::vars())
        .current_dir(PathBuf::from("./skia"))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("gn error");

    if output.status.code() != Some(0) {
        panic!("{:?}", String::from_utf8(output.stdout).unwrap());
    }

    let ninja_command = if on_windows {
        "depot_tools/ninja"
    } else {
        "../depot_tools/ninja"
    };

    assert!(
        Command::new(ninja_command)
            .current_dir(PathBuf::from("./skia"))
            .args(&["-C", output_directory_str])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .expect("failed to run `ninja`, does the directory depot_tools/ exist?")
            .success(),
        "`ninja` returned an error, please check the output for details."
    );

    let current_dir = env::current_dir().unwrap();

    bindgen_gen(build, &current_dir, &config.output_directory)
}

fn bindgen_gen(build: &FinalBuildConfiguration, current_dir: &Path, output_directory: &Path) {
    let mut builder = bindgen::Builder::default()
        .generate_comments(false)
        .layout_tests(true)
        // on macOS some arrays that are used in opaque types get too large to support Debug.
        // (for example High Sierra: [u16; 105])
        // TODO: may reenable when const generics land in stable.
        .derive_debug(false)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .constified_enum(".*Mask")
        .constified_enum(".*Flags")
        .constified_enum(".*Bits")
        .constified_enum("SkCanvas_SaveLayerFlagsSet")
        .constified_enum("GrVkAlloc_Flag")
        .constified_enum("GrGLBackendState")
        .whitelist_function("C_.*")
        .whitelist_function("SkAnnotateRectWithURL")
        .whitelist_function("SkAnnotateNamedDestination")
        .whitelist_function("SkAnnotateLinkToDestination")
        .whitelist_function("SkColorTypeBytesPerPixel")
        .whitelist_function("SkColorTypeIsAlwaysOpaque")
        .whitelist_function("SkColorTypeValidateAlphaType")
        .whitelist_function("SkRGBToHSV")
        // this function does not whitelist (probably because of inlining):
        .whitelist_function("SkColorToHSV")
        .whitelist_function("SkHSVToColor")
        .whitelist_function("SkPreMultiplyARGB")
        .whitelist_function("SkPreMultiplyColor")
        .whitelist_function("SkBlendMode_Name")
        .whitelist_function("SkSwapRB")
        // functions for which the doc generation fails.
        .blacklist_function("SkColorFilter_asComponentTable")
        // core/
        .whitelist_type("SkAutoCanvasRestore")
        .whitelist_type("SkColorSpacePrimaries")
        .whitelist_type("SkContourMeasure")
        .whitelist_type("SkContourMeasureIter")
        .whitelist_type("SkCubicMap")
        .whitelist_type("SkDataTable")
        .whitelist_type("SkDocument")
        .whitelist_type("SkDrawLooper")
        .whitelist_type("SkDynamicMemoryWStream")
        .whitelist_type("SkFontMgr")
        .whitelist_type("SkGraphics")
        .whitelist_type("SkMemoryStream")
        .whitelist_type("SkMultiPictureDraw")
        .whitelist_type("SkPathMeasure")
        .whitelist_type("SkPictureRecorder")
        .whitelist_type("SkVector4")
        .whitelist_type("SkYUVASizeInfo")
        // effects/
        .whitelist_type("SkPath1DPathEffect")
        .whitelist_type("SkLine2DPathEffect")
        .whitelist_type("SkPath2DPathEffect")
        .whitelist_type("SkCornerPathEffect")
        .whitelist_type("SkDashPathEffect")
        .whitelist_type("SkDiscretePathEffect")
        .whitelist_type("SkGradientShader")
        .whitelist_type("SkLayerDrawLooper_Bits")
        .whitelist_type("SkPerlinNoiseShader")
        .whitelist_type("SkTableColorFilter")
        // gpu/
        .whitelist_type("GrGLBackendState")
        // gpu/vk/
        .whitelist_type("GrVkDrawableInfo")
        .whitelist_type("GrVkExtensionFlags")
        .whitelist_type("GrVkFeatureFlags")
        // pathops/
        .whitelist_type("SkPathOp")
        .whitelist_function("Op")
        .whitelist_function("Simplify")
        .whitelist_function("TightBounds")
        .whitelist_function("AsWinding")
        .whitelist_type("SkOpBuilder")
        // utils/
        .whitelist_function("Sk3LookAt")
        .whitelist_function("Sk3Perspective")
        .whitelist_function("Sk3MapPts")
        .whitelist_function("SkUnitCubicInterp")
        .whitelist_type("Sk3DView")
        .whitelist_type("SkInterpolator")
        .whitelist_type("SkParsePath")
        .whitelist_type("SkShadowUtils")
        .whitelist_type("SkShadowFlags")
        .whitelist_type("SkTextUtils")
        // modules/skshaper/
        .whitelist_type("SkShaper")
        .whitelist_type("RustRunHandler")
        // misc
        .whitelist_var("SK_Color.*")
        .whitelist_var("kAll_GrBackendState")
        //
        .use_core()
        .clang_arg("-std=c++14")
        // required for macOS LLVM 8 to pick up C++ headers:
        .clang_args(&["-x", "c++"])
        .clang_arg("-v");

    for opaque_type in OPAQUE_TYPES {
        builder = builder.opaque_type(opaque_type)
    }

    let mut cc_build = Build::new();

    for source in &build.binding_sources {
        cc_build.file(source);
        let source = source.to_str().unwrap();
        cargo::add_dependent_path(source);
        builder = builder.header(source);
    }

    // TODO: may put the include paths into the FinalBuildConfiguration?

    let include_path = current_dir.join("skia");
    cargo::add_dependent_path(include_path.join("include"));

    builder = builder.clang_arg(format!("-I{}", include_path.display()));
    cc_build.include(include_path);

    let definitions = {
        let skia_definitions = {
            let ninja_file = output_directory.join("obj").join("skia.ninja");
            let contents = fs::read_to_string(ninja_file).unwrap();
            definitions::from_ninja(contents)
        };

        definitions::combine(skia_definitions, build.definitions.clone())
    };

    for (name, value) in &definitions {
        match value {
            Some(value) => {
                cc_build.define(name, value.as_str());
                builder = builder.clang_arg(format!("-D{}={}", name, value));
            }
            None => {
                cc_build.define(name, "");
                builder = builder.clang_arg(format!("-D{}", name));
            }
        }
    }

    cc_build.cpp(true).out_dir(output_directory);

    if !cfg!(windows) {
        cc_build.flag("-std=c++14");
    }

    let target = cargo::target();
    match target.as_strs() {
        (arch, "linux", "android", _) => {
            let target = &target.to_string();
            cc_build.target(target);
            for arg in android::additional_clang_args(target, arch) {
                builder = builder.clang_arg(arg);
            }
        }
        (arch, "apple", "ios", _) => {
            for arg in ios::additional_clang_args(arch) {
                builder = builder.clang_arg(arg);
            }
        }
        _ => {}
    }

    println!("COMPILING BINDINGS: {:?}", build.binding_sources);
    cc_build.compile(lib::SKIA_BINDINGS);

    println!("GENERATING BINDINGS");
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

const OPAQUE_TYPES: &[&str] = &[
    // Types for which the binding generator pulls in stuff that can not be compiled.
    "SkDeferredDisplayList",
    "SkDeferredDisplayList_PendingPathsMap",
    // Types for which a bindgen layout is wrong causing types that contain
    // fields of them to fail their layout test.

    // Windows:
    "std::atomic",
    "std::function",
    "std::unique_ptr",
    "SkAutoTMalloc",
    "SkTHashMap",
    // Ubuntu 18 LLVM 6: all types derived from SkWeakRefCnt
    "SkWeakRefCnt",
    "GrContext",
    "GrContextThreadSafeProxy",
    "GrContext_Base",
    "GrGLInterface",
    "GrImageContext",
    "GrRecordingContext",
    "GrSurfaceProxy",
    "Sk2DPathEffect",
    "SkCornerPathEffect",
    "SkDataTable",
    "SkDiscretePathEffect",
    "SkDrawable",
    "SkLine2DPathEffect",
    "SkPath2DPathEffect",
    "SkPathRef_GenIDChangeListener",
    "SkPicture",
    "SkPixelRef",
    "SkSurface",
];

mod prerequisites {
    use crate::build_support::{cargo, utils};
    use flate2::read::GzDecoder;
    use std::ffi::OsStr;
    use std::fs;
    use std::io::Cursor;
    use std::path::Component;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    pub fn locate_python2_cmd() -> &'static str {
        const PYTHON_CMDS: [&str; 2] = ["python", "python2"];
        for python in PYTHON_CMDS.as_ref() {
            println!("Probing '{}'", python);
            if let Some(true) = is_python_version_2(python) {
                return python;
            }
        }

        panic!(">>>>> Probing for Python 2 failed, please make sure that it's available in PATH, probed executables are: {:?} <<<<<", PYTHON_CMDS);
    }

    /// Returns true if the given python executable is python version 2.
    /// or None if the executable was not found.
    pub fn is_python_version_2(exe: impl AsRef<str>) -> Option<bool> {
        Command::new(exe.as_ref())
            .arg("--version")
            .output()
            .map(|output| {
                let mut str = String::from_utf8(output.stdout).unwrap();
                if str.is_empty() {
                    // Python2 seems to push the version to stderr.
                    str = String::from_utf8(output.stderr).unwrap()
                }
                // Don't parse version output, for example output
                // might be "Python 2.7.15+"
                str.starts_with("Python 2.")
            })
            .ok()
    }

    /// Resolve the skia and depot_tools subdirectory contents, either by checking out the
    /// submodules, or when the build.rs was called outside of the git repository,
    /// by downloading and unpacking them from GitHub.
    pub fn resolve_dependencies() {
        if cargo::is_crate() {
            // we are in a crate.
            download_dependencies();
        } else {
            // we are not in a crate, assuming we are in our git repo.
            // so just update all submodules.
            assert!(
                Command::new("git")
                    .args(&["submodule", "update", "--init", "--depth", "1"])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .unwrap()
                    .success(),
                "`git submodule update` failed"
            );
        }
    }

    /// Downloads the skia and depot_tools from their repositories.
    ///
    /// The hashes are taken from the Cargo.toml section [package.metadata].
    fn download_dependencies() {
        let metadata = cargo::get_metadata();

        for dep in dependencies() {
            let repo_url = dep.url;
            let repo_name = dep.repo;

            let dir = PathBuf::from(repo_name);

            // directory exists => assume that the download of the archive was successful.
            if dir.exists() {
                continue;
            }

            // hash available?
            let (_, short_hash) = metadata
                .iter()
                .find(|(n, _)| n == repo_name)
                .expect("metadata entry not found");

            // remove unpacking directory, github will format it to repo_name-hash
            let unpack_dir = &PathBuf::from(format!("{}-{}", repo_name, short_hash));
            if unpack_dir.is_dir() {
                fs::remove_dir_all(unpack_dir).unwrap();
            }

            // download
            let archive_url = &format!("{}/{}", repo_url, short_hash);
            println!("DOWNLOADING: {}", archive_url);
            let archive =
                utils::download(archive_url).expect(&format!("Failed to download {}", archive_url));

            // unpack
            {
                let tar = GzDecoder::new(Cursor::new(archive));
                let mut archive = tar::Archive::new(tar);
                let dir = std::env::current_dir().unwrap();
                for entry in archive.entries().expect("failed to iterate over archive") {
                    let mut entry = entry.unwrap();
                    let path = entry.path().unwrap();
                    let mut components = path.components();
                    let root = components.next().unwrap();
                    // skip pax headers.
                    if root.as_os_str() == unpack_dir.as_os_str()
                        && (dep.path_filter)(components.as_path())
                    {
                        entry.unpack_in(&dir).unwrap();
                    }
                }
            }

            // move unpack directory to the target repository directory
            fs::rename(unpack_dir, repo_name).expect("failed to move directory");
        }
    }

    // Specifies where to download Skia and Depot Tools archives from.
    //
    // We use codeload.github.com, otherwise the short hash will be expanded to a full hash as the root
    // directory inside the tar.gz, and we run into filesystem path length restrictions
    // with Skia.
    struct Dependency {
        pub repo: &'static str,
        pub url: &'static str,
        pub path_filter: fn(&Path) -> bool,
    }

    fn dependencies() -> &'static [Dependency] {
        return &[
            Dependency {
                repo: "skia",
                url: "https://codeload.github.com/google/skia/tar.gz",
                path_filter: filter_skia,
            },
            Dependency {
                repo: "depot_tools",
                url: "https://codeload.github.com/rust-skia/depot_tools/tar.gz",
                path_filter: filter_depot_tools,
            },
        ];

        // infra/ contains very long filenames which may hit the max path restriction on Windows.
        // https://github.com/rust-skia/rust-skia/issues/169
        fn filter_skia(p: &Path) -> bool {
            match p.components().next() {
                Some(Component::Normal(name)) if name == OsStr::new("infra") => false,
                _ => true,
            }
        }

        // we need only ninja from depot_tools.
        // https://github.com/rust-skia/rust-skia/pull/165
        fn filter_depot_tools(p: &Path) -> bool {
            p.to_str().unwrap().starts_with("ninja")
        }
    }
}

use bindgen::EnumVariation;
pub use definitions::{Definition, Definitions};

pub(crate) mod definitions {
    use std::collections::HashSet;

    /// A preprocessor definition.
    pub type Definition = (String, Option<String>);
    /// A container for a number of preprocessor definitions.
    pub type Definitions = Vec<Definition>;

    /// Parse a defines = line from a ninja build file.
    pub fn from_ninja(ninja_file: impl AsRef<str>) -> Definitions {
        let lines: Vec<&str> = ninja_file.as_ref().lines().collect();
        let defines = {
            let prefix = "defines = ";
            let defines = lines
                .into_iter()
                .find(|s| s.starts_with(prefix))
                .expect("missing a line with the prefix 'defines =' in a .ninja file");
            &defines[prefix.len()..]
        };
        let defines: Vec<&str> = {
            let prefix = "-D";
            defines
                .split_whitespace()
                .into_iter()
                .map(|d| {
                    if d.starts_with(prefix) {
                        &d[prefix.len()..]
                    } else {
                        panic!("missing '-D' prefix from a definition")
                    }
                })
                .collect()
        };
        defines
            .into_iter()
            .map(|d| {
                let items: Vec<&str> = d.splitn(2, "=").collect();
                match items.len() {
                    1 => (items[0].to_string(), None),
                    2 => (items[0].to_string(), Some(items[1].to_string())),
                    _ => panic!("internal error"),
                }
            })
            .collect()
    }

    pub fn combine(a: Definitions, b: Definitions) -> Definitions {
        compress(a.into_iter().chain(b.into_iter()).collect())
    }

    pub fn compress(mut definitions: Definitions) -> Definitions {
        let mut uniques = HashSet::new();
        definitions.retain(|e| uniques.insert(e.0.clone()));
        definitions
    }
}
