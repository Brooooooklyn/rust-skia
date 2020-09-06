//! Full build support for the Skia library, SkiaBindings library and bindings.rs file.

use crate::build_support::{android, binaries, cargo, clang, ios, llvm, vs, xcode};
use bindgen::{CodegenConfig, EnumVariation};
use cc::Build;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

/// The libraries to link with.
mod lib {
    pub const SKIA: &str = "skia";
    pub const SKIA_BINDINGS: &str = "skia-bindings";
    pub const SKSHAPER: &str = "skshaper";
    pub const SKPARAGRAPH: &str = "skparagraph";
}

/// Feature identifiers define the additional configuration parts of the binaries to download.
mod feature_id {
    pub const GL: &str = "gl";
    pub const VULKAN: &str = "vulkan";
    pub const METAL: &str = "metal";
    pub const D3D: &str = "d3d";
    pub const TEXTLAYOUT: &str = "textlayout";
}

/// The defaults for the Skia build configuration.
impl Default for BuildConfiguration {
    fn default() -> Self {
        let skia_debug = matches!(cargo::env_var("SKIA_DEBUG"), Some(v) if v != "0");

        BuildConfiguration {
            on_windows: cargo::host().is_windows(),
            skia_debug,
            features: Features {
                gl: cfg!(feature = "gl"),
                vulkan: cfg!(feature = "vulkan"),
                metal: cfg!(feature = "metal"),
                d3d: cfg!(feature = "d3d"),
                text_layout: cfg!(feature = "textlayout"),
                animation: false,
                dng: false,
                particles: false,
            },
            definitions: Vec::new(),
        }
    }
}

/// The build configuration for Skia.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildConfiguration {
    /// Do we build _on_ a Windows OS?
    on_windows: bool,

    /// Build Skia in a debug configuration?
    skia_debug: bool,

    /// The Skia feature set to compile.
    features: Features,

    /// Additional preprocessor definitions that will override predefined ones.
    definitions: Definitions,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Features {
    /// Build with OpenGL / EGL support?
    pub gl: bool,

    /// Build with Vulkan support?
    pub vulkan: bool,

    /// Build with Metal support?
    pub metal: bool,

    /// Build with Direct3D support?
    pub d3d: bool,

    /// Features related to text layout. Modules skshaper and skparagraph.
    pub text_layout: bool,

    /// Build with animation support (yet unsupported, no wrappers).
    pub animation: bool,

    /// Support DNG file format (currently unsupported because of build errors).
    pub dng: bool,

    /// Build the particles module (unsupported, no wrappers).
    pub particles: bool,
}

impl Features {
    pub fn gpu(&self) -> bool {
        self.gl || self.vulkan || self.metal || self.d3d
    }

    /// Feature Ids used to look up prebuilt binaries.
    pub fn ids(&self) -> Vec<&str> {
        let mut feature_ids = Vec::new();

        if self.gl {
            feature_ids.push(feature_id::GL);
        }
        if self.vulkan {
            feature_ids.push(feature_id::VULKAN);
        }
        if self.metal {
            feature_ids.push(feature_id::METAL);
        }
        if self.d3d {
            feature_ids.push(feature_id::D3D);
        }
        if self.text_layout {
            feature_ids.push(feature_id::TEXTLAYOUT);
        }

        feature_ids
    }
}

/// This is the final, low level build configuration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FinalBuildConfiguration {
    /// The Skia source directory.
    pub skia_source_dir: PathBuf,

    /// The name value pairs passed as arguments to gn.
    pub gn_args: Vec<(String, String)>,

    /// ninja files that need to be parsed for further definitions.
    pub ninja_files: Vec<PathBuf>,

    /// The additional definitions (cloned from the definitions of
    /// the BuildConfiguration).
    pub definitions: Definitions,

    /// The binding source files to compile.
    pub binding_sources: Vec<PathBuf>,
}

impl FinalBuildConfiguration {
    #[allow(clippy::cognitive_complexity)]
    pub fn from_build_configuration(
        build: &BuildConfiguration,
        skia_source_dir: &Path,
    ) -> FinalBuildConfiguration {
        let features = &build.features;

        let gn_args = {
            fn quote(s: &str) -> String {
                format!("\"{}\"", s)
            }

            let mut args: Vec<(&str, String)> = vec![
                (
                    "is_official_build",
                    if build.skia_debug { no() } else { yes() },
                ),
                ("is_debug", if build.skia_debug { yes() } else { no() }),
                ("skia_enable_gpu", if features.gpu() { yes() } else { no() }),
                ("skia_use_gl", if features.gl { yes() } else { no() }),
                ("skia_use_system_libjpeg_turbo", no()),
                ("skia_use_system_libpng", no()),
                ("skia_use_libwebp_encode", no()),
                ("skia_use_libwebp_decode", no()),
                ("skia_use_system_zlib", no()),
                ("skia_use_xps", no()),
                ("skia_use_dng_sdk", if features.dng { yes() } else { no() }),
            ];

            if features.vulkan {
                args.push(("skia_use_vulkan", yes()));
                args.push(("skia_enable_spirv_validation", no()));
            }

            if features.metal {
                args.push(("skia_use_metal", yes()));
            }

            if features.d3d {
                args.push(("skia_use_direct3d", yes()))
            }

            // further flags that limit the components of Skia debug builds.
            if build.skia_debug {
                args.push(("skia_enable_atlas_text", no()));
                args.push(("skia_enable_spirv_validation", no()));
                args.push(("skia_enable_tools", no()));
                args.push(("skia_enable_vulkan_debug_layers", no()));
                args.push(("skia_use_libheif", no()));
                args.push(("skia_use_lua", no()));
            }

            if features.text_layout {
                args.extend(vec![
                    ("skia_enable_skshaper", yes()),
                    ("skia_use_icu", yes()),
                    ("skia_use_system_icu", no()),
                    ("skia_use_harfbuzz", yes()),
                    ("skia_pdf_subset_harfbuzz", yes()),
                    ("skia_use_system_harfbuzz", no()),
                    ("skia_use_sfntly", no()),
                    ("skia_enable_skparagraph", yes()),
                    // note: currently, tests need to be enabled, because modules/skparagraph
                    // is not included in the default dependency configuration.
                    // ("paragraph_tests_enabled", no()),
                ]);
            } else {
                args.push(("skia_use_icu", no()));
            }

            let mut flags: Vec<&str> = vec![];
            let mut use_expat = true;
            let mut compiler_executables = ("clang", "clang++");

            // target specific gn args.
            let target = cargo::target();
            match target.as_strs() {
                (_, _, "windows", Some("msvc")) if build.on_windows => {
                    if let Some(win_vc) = vs::resolve_win_vc() {
                        args.push(("win_vc", quote(win_vc.to_str().unwrap())))
                    }
                    // Code on MSVC needs to be compiled differently (e.g. with /MT or /MD) depending on the runtime being linked.
                    // (See https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)
                    // When static feature is enabled (target-feature=+crt-static) the C runtime should be statically linked
                    // and the compiler has to place the library name LIBCMT.lib into the .obj
                    // See https://docs.microsoft.com/en-us/cpp/build/reference/md-mt-ld-use-run-time-library?view=vs-2019
                    if cargo::target_crt_static() {
                        flags.push("/MT");
                    }
                    // otherwise the C runtime should be linked dynamically
                    else {
                        flags.push("/MD");
                    }
                    // Tell Skia's build system where LLVM is supposed to be located.
                    if let Some(llvm_home) = llvm::win::find_llvm_home() {
                        args.push(("clang_win", quote(&llvm_home)));
                    } else {
                        panic!(
                            "Unable to locate LLVM installation. skia-bindings can not be built."
                        );
                    }
                }
                (arch, "linux", "android", _) | (arch, "linux", "androideabi", _) => {
                    args.push(("ndk", quote(&android::ndk())));
                    // TODO: make API-level configurable?
                    args.push(("ndk_api", android::API_LEVEL.into()));
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
                ("wasm32", "unknown", "emscripten", _) => {
                    args.push(("target_cpu", quote("wasm")));
                    compiler_executables = ("emcc", "em++");
                }
                _ => {}
            }

            if use_expat {
                args.push(("skia_use_expat", yes()));
                args.push(("skia_use_system_expat", no()));
            } else {
                args.push(("skia_use_expat", no()));
            }

            {
                args.push(("cc", quote(compiler_executables.0)));
                args.push(("cxx", quote(compiler_executables.1)));
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

        let ninja_files = {
            let mut files = Vec::new();
            files.push("obj/skia.ninja".into());
            if features.text_layout {
                files.extend(vec![
                    "obj/modules/skshaper/skshaper.ninja".into(),
                    "obj/modules/skparagraph/skparagraph.ninja".into(),
                ]);
            }
            files
        };

        let binding_sources = {
            let mut sources: Vec<PathBuf> = Vec::new();
            sources.push("src/bindings.cpp".into());
            if features.gl {
                sources.push("src/gl.cpp".into());
            }
            if features.vulkan {
                sources.push("src/vulkan.cpp".into());
            }
            if features.metal {
                sources.push("src/metal.cpp".into());
            }
            if features.d3d {
                sources.push("src/d3d.cpp".into());
            }
            if features.gpu() {
                sources.push("src/gpu.cpp".into());
            }
            if features.text_layout {
                sources.extend(vec!["src/shaper.cpp".into(), "src/paragraph.cpp".into()]);
            }
            sources.push("src/svg.cpp".into());
            sources
        };

        FinalBuildConfiguration {
            skia_source_dir: skia_source_dir.into(),
            gn_args,
            ninja_files,
            definitions: build.definitions.clone(),
            binding_sources,
        }
    }
}

fn yes() -> String {
    "true".into()
}
fn no() -> String {
    "false".into()
}

/// The configuration of the resulting binaries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BinariesConfiguration {
    /// The feature identifiers we built with.
    pub feature_ids: Vec<String>,

    /// The output directory of the libraries we build and we need to inform cargo about.
    pub output_directory: PathBuf,

    /// The TARGET specific link libraries we need to inform cargo about.
    pub link_libraries: Vec<String>,

    /// The static Skia libraries skia-bindings provides and dependent projects need to link with.
    pub built_libraries: Vec<String>,

    /// Additional files relative to the output_directory
    /// that are needed to build dependent projects.
    pub additional_files: Vec<PathBuf>,

    /// `true` if the skia libraries are built with debugging information.
    pub skia_debug: bool,
}

const SKIA_OUTPUT_DIR: &str = "skia";
const ICUDTL_DAT: &str = "icudtl.dat";

impl BinariesConfiguration {
    /// Build a binaries configuration based on the current environment cargo
    /// supplies us with and a Skia build configuration.
    pub fn from_cargo_env(build: &BuildConfiguration) -> Self {
        let features = &build.features;
        let target = cargo::target();

        let mut built_libraries = Vec::new();
        let mut additional_files = Vec::new();
        let feature_ids = features.ids();

        if features.text_layout {
            additional_files.push(ICUDTL_DAT.into());
            built_libraries.push(lib::SKPARAGRAPH.into());
            built_libraries.push(lib::SKSHAPER.into());
        }

        let mut link_libraries = Vec::new();

        match target.as_strs() {
            (_, "unknown", "linux", Some("gnu")) => {
                link_libraries.extend(vec!["stdc++", "fontconfig", "freetype"]);
                if features.gl {
                    link_libraries.push("GL");
                }
            }
            (_, "apple", "darwin", _) => {
                link_libraries.extend(vec!["c++", "framework=ApplicationServices"]);
                if features.gl {
                    link_libraries.push("framework=OpenGL");
                }
                if features.metal {
                    link_libraries.push("framework=Metal");
                    link_libraries.push("framework=Foundation");
                }
            }
            (_, _, "windows", Some("msvc")) => {
                link_libraries.extend(&["usp10", "ole32", "user32", "gdi32", "fontsub"]);
                if features.gl {
                    link_libraries.push("opengl32");
                }
                if features.d3d {
                    link_libraries.extend(&["d3d12", "dxgi", "d3dcompiler"]);
                }
            }
            (_, "linux", "android", _) | (_, "linux", "androideabi", _) => {
                link_libraries.extend(android::link_libraries(features));
            }
            (_, "apple", "ios", _) => {
                link_libraries.extend(ios::link_libraries(features));
            }
            ("wasm32", "unknown", "emscripten", _) => {
                link_libraries.extend(vec!["stdc++", "bz2", "GL", "fontconfig", "freetype"]);
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
            feature_ids: feature_ids.into_iter().map(|f| f.to_string()).collect(),
            output_directory,
            link_libraries: link_libraries
                .into_iter()
                .map(|lib| lib.to_string())
                .collect(),
            built_libraries,
            additional_files,
            skia_debug: build.skia_debug,
        }
    }

    /// Inform cargo that the library files of the given configuration are available and
    /// can be used as dependencies.
    pub fn commit_to_cargo(&self) {
        println!(
            "cargo:rustc-link-search={}",
            self.output_directory.to_str().unwrap()
        );

        // On Linux, the order is significant, first the static libraries we built, and then
        // the system libraries.

        let target = cargo::target();

        for lib in &self.built_libraries {
            // Prefixing the libraries we built with `static=` causes linker errors on Windows.
            // https://github.com/rust-skia/rust-skia/pull/354
            let kind_prefix = {
                if target.is_windows() {
                    ""
                } else {
                    "static="
                }
            };

            cargo::add_link_lib(format!("{}{}", kind_prefix, lib));
        }

        cargo::add_link_libs(&self.link_libraries);
    }

    pub fn key(&self, repository_short_hash: &str) -> String {
        binaries::key(repository_short_hash, &self.feature_ids, self.skia_debug)
    }
}

/// The full build of Skia, skia-bindings, and the generation of bindings.rs.
pub fn build(build: &FinalBuildConfiguration, config: &BinariesConfiguration) {
    let python2 = &prerequisites::locate_python2_cmd();
    println!("Python 2 found: {:?}", python2);
    let ninja = fetch_dependencies(&python2);
    configure_skia(build, config, &python2, None);
    build_skia(build, config, &ninja);
}

/// Build Skia without any network access.
///
/// An offline build expects the Skia source tree including all third party dependencies
/// to be available.
pub fn build_offline(
    build: &FinalBuildConfiguration,
    config: &BinariesConfiguration,
    ninja_command: Option<&Path>,
    gn_command: Option<&Path>,
) {
    let python2 = prerequisites::locate_python2_cmd();
    configure_skia(&build, &config, &python2, gn_command);
    build_skia(
        &build,
        &config,
        ninja_command.unwrap_or(&ninja::default_exe_name()),
    );
}

/// Prepares the build and returns the ninja command to use for building Skia.
pub fn fetch_dependencies(python2: &Path) -> PathBuf {
    prerequisites::resolve_dependencies();

    // call Skia's git-sync-deps

    println!("Synchronizing Skia dependencies");

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

    env::current_dir()
        .unwrap()
        .join("depot_tools")
        .join(ninja::default_exe_name())
}

/// Configures Skia by calling gn
pub fn configure_skia(
    build: &FinalBuildConfiguration,
    config: &BinariesConfiguration,
    python2: &Path,
    gn_command: Option<&Path>,
) {
    let gn_args = build
        .gn_args
        .iter()
        .map(|(name, value)| name.clone() + "=" + value)
        .collect::<Vec<String>>()
        .join(" ");

    let gn_command = gn_command
        .map(|p| p.to_owned())
        .unwrap_or_else(|| build.skia_source_dir.join("bin").join("gn"));

    println!("Skia args: {}", &gn_args);

    let output = Command::new(gn_command)
        .args(&[
            "gen",
            config.output_directory.to_str().unwrap(),
            &format!("--script-executable={}", python2.to_str().unwrap()),
            &format!("--args={}", gn_args),
        ])
        .envs(env::vars())
        .current_dir(&build.skia_source_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("gn error");

    if output.status.code() != Some(0) {
        panic!("{:?}", String::from_utf8(output.stdout).unwrap());
    }
}

/// Builds Skia.
///
/// This function assumes that all prerequisites are in place and that the output directory
/// contains a fully configured Skia source tree generated by gn.
pub fn build_skia(
    build: &FinalBuildConfiguration,
    config: &BinariesConfiguration,
    ninja_command: &Path,
) {
    let ninja_status = Command::new(ninja_command)
        .args(&["-C", config.output_directory.to_str().unwrap()])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    assert!(
        ninja_status
            .expect("failed to run `ninja`, does the directory depot_tools/ exist?")
            .success(),
        "`ninja` returned an error, please check the output for details."
    );

    generate_bindings(build, &config.output_directory)
}

fn generate_bindings(build: &FinalBuildConfiguration, output_directory: &Path) {
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
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(ParseCallbacks))
        .raw_line("#![allow(clippy::all)]")
        // GrVkBackendContext contains u128 fields on macOS
        .raw_line("#![allow(improper_ctypes)]")
        .whitelist_function("C_.*")
        .constified_enum(".*Mask")
        .constified_enum(".*Flags")
        .constified_enum(".*Bits")
        .constified_enum("SkCanvas_SaveLayerFlagsSet")
        .constified_enum("GrVkAlloc_Flag")
        .constified_enum("GrGLBackendState")
        // not used:
        .blacklist_type("SkPathRef_Editor")
        .blacklist_function("SkPathRef_Editor_Editor")
        // private types that pull in inline functions that cannot be linked:
        // https://github.com/rust-skia/rust-skia/issues/318
        .raw_line("pub enum GrContext_Base {}")
        .blacklist_type("GrContext_Base")
        .blacklist_function("GrContext_Base_.*")
        .raw_line("pub enum GrImageContext {}")
        .blacklist_type("GrImageContext")
        .raw_line("pub enum GrImageContextPriv {}")
        .blacklist_type("GrImageContextPriv")
        .raw_line("pub enum GrContextThreadSafeProxy {}")
        .blacklist_type("GrContextThreadSafeProxy")
        .raw_line("pub enum GrContextThreadSafeProxyPriv {}")
        .blacklist_type("GrContextThreadSafeProxyPriv")
        .raw_line("pub enum GrRecordingContext {}")
        .blacklist_type("GrRecordingContext")
        .raw_line("pub enum GrRecordingContextPriv {}")
        .blacklist_type("GrRecordingContextPriv")
        .raw_line("pub enum GrContextPriv {}")
        .blacklist_type("GrContextPriv")
        .blacklist_function("GrContext_priv.*")
        .blacklist_function("SkDeferredDisplayList_priv.*")
        .raw_line("pub enum SkVerticesPriv {}")
        .blacklist_type("SkVerticesPriv")
        .blacklist_function("SkVertices_priv.*")
        // Vulkan reexports that got swallowed by making them opaque.
        // (these can not be whitelisted by a extern "C" function)
        .whitelist_type("VkPhysicalDeviceFeatures")
        .whitelist_type("VkPhysicalDeviceFeatures2")
        // misc
        .whitelist_var("SK_Color.*")
        .whitelist_var("kAll_GrBackendState")
        //
        // don't generate destructors: https://github.com/rust-skia/rust-skia/issues/318
        .with_codegen_config({
            let mut config = CodegenConfig::default();
            config.remove(CodegenConfig::DESTRUCTORS);
            config
        })
        //
        .use_core()
        .clang_arg("-std=c++17")
        // required for macOS LLVM 8 to pick up C++ headers:
        .clang_args(&["-x", "c++"])
        .clang_arg("-v");

    for function in WHITELISTED_FUNCTIONS {
        builder = builder.whitelist_function(function)
    }

    for opaque_type in OPAQUE_TYPES {
        builder = builder.opaque_type(opaque_type)
    }

    for t in BLACKLISTED_TYPES {
        builder = builder.blacklist_type(t);
    }

    let mut cc_build = Build::new();

    for source in &build.binding_sources {
        cc_build.file(source);
        let source = source.to_str().unwrap();
        cargo::rerun_if_changed(source);
        builder = builder.header(source);
    }

    let include_path = &build.skia_source_dir;
    cargo::rerun_if_changed(include_path.join("include"));

    builder = builder.clang_arg(format!("-I{}", include_path.display()));
    cc_build.include(include_path);

    let definitions = {
        let mut definitions = Vec::new();

        for ninja_file in &build.ninja_files {
            let ninja_file = output_directory.join(ninja_file);
            let contents = fs::read_to_string(ninja_file).unwrap();
            definitions = definitions::combine(definitions, definitions::from_ninja(contents))
        }

        definitions::combine(definitions, build.definitions.clone())
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
        cc_build.flag("-std=c++17");
    }

    let target = cargo::target();
    match target.as_strs() {
        (_, "apple", "darwin", _) => {
            if let Some(sdk) = xcode::get_sdk_path("macosx") {
                builder = builder.clang_arg(format!("-isysroot{}", sdk.to_str().unwrap()));
            } else {
                cargo::warning("failed to get macosx SDK path")
            }
        }
        (arch, "linux", "android", _) | (arch, "linux", "androideabi", _) => {
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
        ("wasm32", "unknown", "emscripten", _) => {
            builder = builder.clang_arg("--target=wasm32-unknown-emscripten");
            // Add C++ includes (otherwise build will fail with <cmath> not found)
            builder = builder.clang_arg("-I/usr/share/emscripten/system/include/libcxx");
            // visibility=default, otherwise some types may be missing:
            // https://github.com/rust-lang/rust-bindgen/issues/751#issuecomment-555735577
            builder = builder.clang_arg("-fvisibility=default");
        }
        _ => {}
    }

    println!("COMPILING BINDINGS: {:?}", build.binding_sources);
    // we add skia-bindings later on.
    cc_build.cargo_metadata(false);
    cc_build.compile(lib::SKIA_BINDINGS);

    println!("GENERATING BINDINGS");
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from("src");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

const WHITELISTED_FUNCTIONS: &[&str] = &[
    "SkAnnotateRectWithURL",
    "SkAnnotateNamedDestination",
    "SkAnnotateLinkToDestination",
    "SkColorTypeBytesPerPixel",
    "SkColorTypeIsAlwaysOpaque",
    "SkColorTypeValidateAlphaType",
    "SkRGBToHSV",
    // this function does not whitelist (probably because of inlining):
    "SkColorToHSV",
    "SkHSVToColor",
    "SkPreMultiplyARGB",
    "SkPreMultiplyColor",
    "SkBlendMode_AsCoeff",
    "SkBlendMode_Name",
    "SkSwapRB",
    // functions for which the doc generation fails
    "SkColorFilter_asComponentTable",
    // pathops/
    "Op",
    "Simplify",
    "TightBounds",
    "AsWinding",
    // utils/
    "Sk3LookAt",
    "Sk3Perspective",
    "Sk3MapPts",
    "SkUnitCubicInterp",
];

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
    "GrGLInterface",
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
    // Types not needed (for now):
    "SkDeque",
    "SkDeque_Iter",
    "GrGLInterface_Functions",
    // SkShaper (m77) Trivial*Iterator classes create two vtable pointers.
    "SkShaper_TrivialBiDiRunIterator",
    "SkShaper_TrivialFontRunIterator",
    "SkShaper_TrivialLanguageRunIterator",
    "SkShaper_TrivialScriptRunIterator",
    // skparagraph
    "std::vector",
    "std::u16string",
    // skparagraph (m78), (layout fails on macOS and Linux, not sure why, looks like an obscure alignment problem)
    "skia::textlayout::FontCollection",
    // skparagraph (m79), std::map is used in LineMetrics
    "std::map",
    // Vulkan reexports with the wrong field naming conventions.
    "VkPhysicalDeviceFeatures",
    "VkPhysicalDeviceFeatures2",
    // Since Rust 1.39 beta (TODO: investigate why, and re-test when 1.39 goes stable).
    "GrContextOptions_PersistentCache",
    "GrContextOptions_ShaderErrorHandler",
    "Sk1DPathEffect",
    "SkBBoxHierarchy", // vtable
    "SkBBHFactory",
    "SkBitmap_Allocator",
    "SkBitmap_HeapAllocator",
    "SkColorFilter",
    "SkDeque_F2BIter",
    "SkDrawLooper",
    "SkDrawLooper_Context",
    "SkDrawable_GpuDrawHandler",
    "SkFlattenable",
    "SkFontMgr",
    "SkFontStyleSet",
    "SkMaskFilter",
    "SkPathEffect",
    "SkPicture_AbortCallback",
    "SkPixelRef_GenIDChangeListener",
    "SkRasterHandleAllocator",
    "SkRefCnt",
    "SkShader",
    "SkStream",
    "SkStreamAsset",
    "SkStreamMemory",
    "SkStreamRewindable",
    "SkStreamSeekable",
    "SkTypeface_LocalizedStrings",
    "SkWStream",
    "GrVkMemoryAllocator",
    "SkShaper",
    "SkShaper_BiDiRunIterator",
    "SkShaper_FontRunIterator",
    "SkShaper_LanguageRunIterator",
    "SkShaper_RunHandler",
    "SkShaper_RunIterator",
    "SkShaper_ScriptRunIterator",
    "SkContourMeasure",
    "SkDocument",
    // m81: tuples:
    "SkRuntimeEffect_EffectResult",
    "SkRuntimeEffect_ByteCodeResult",
    "SkRuntimeEffect_SpecializeResult",
    // m81: derives from std::string
    "SkSL::String",
    "std::basic_string",
    "std::basic_string_value_type",
    // m81: wrong size on macOS and Linux
    "SkRuntimeEffect",
    "GrShaderCaps",
    // more stuff we don't need that was tracked down fixing:
    // https://github.com/rust-skia/rust-skia/issues/318
    // referred from SkPath, but not used:
    "SkPathRef",
    "SkMutex",
    // m82: private
    "SkIDChangeListener",
];

const BLACKLISTED_TYPES: &[&str] = &[
    // modules/skparagraph
    //   pulls in a std::map<>, which we treat as opaque, but bindgen creates wrong bindings for
    //   std::_Tree* types
    "std::_Tree.*",
    "std::map.*",
    //   debug builds:
    "SkLRUCache",
    "SkLRUCache_Entry",
    //   not used at all:
    "std::vector.*",
    // too much template magic:
    "SkRuntimeEffect_ConstIterable.*",
    // Linux LLVM9 c++17
    "std::_Rb_tree.*",
    // Linux LLVM9 c++17 with SKIA_DEBUG=1
    "std::__cxx.*",
    "std::array.*",
];

#[derive(Debug)]
struct ParseCallbacks;

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
    /// Allows to rename an enum variant, replacing `_original_variant_name`.
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        enum_name.and_then(|enum_name| {
            ENUM_TABLE
                .iter()
                .find(|n| n.0 == enum_name)
                .map(|(_, replacer)| replacer(enum_name, original_variant_name))
        })
    }
}

type EnumEntry = (&'static str, fn(&str, &str) -> String);

const ENUM_TABLE: &[EnumEntry] = &[
    //
    // codec/
    //
    ("DocumentStructureType", rewrite::k_xxx),
    ("ZeroInitialized", rewrite::k_xxx_name),
    ("SelectionPolicy", rewrite::k_xxx),
    //
    // core/ effects/
    //
    ("SkApplyPerspectiveClip", rewrite::k_xxx),
    ("SkBlendMode", rewrite::k_xxx),
    ("SkBlendModeCoeff", rewrite::k_xxx),
    ("SkBlurStyle", rewrite::k_xxx_name),
    ("SkClipOp", rewrite::k_xxx),
    ("SkColorChannel", rewrite::k_xxx),
    ("SkCoverageMode", rewrite::k_xxx),
    ("SkEncodedImageFormat", rewrite::k_xxx),
    ("SkEncodedOrigin", rewrite::k_xxx_name),
    ("SkFilterQuality", rewrite::k_xxx_name),
    ("SkFontHinting", rewrite::k_xxx),
    ("SkAlphaType", rewrite::k_xxx_name),
    ("SkYUVColorSpace", rewrite::k_xxx_name),
    ("SkPathFillType", rewrite::k_xxx),
    ("SkPathConvexityType", rewrite::k_xxx),
    ("SkPathDirection", rewrite::k_xxx),
    ("SkPathVerb", rewrite::k_xxx),
    ("SkPathOp", rewrite::k_xxx_name),
    ("SkTileMode", rewrite::k_xxx),
    // SkPaint_Style
    // SkStrokeRec_Style
    // SkPath1DPathEffect_Style
    ("Style", rewrite::k_xxx_name_opt),
    // SkPaint_Cap
    ("Cap", rewrite::k_xxx_name),
    // SkPaint_Join
    ("Join", rewrite::k_xxx_name),
    // SkStrokeRec_InitStyle
    ("InitStyle", rewrite::k_xxx_name),
    // SkBlurImageFilter_TileMode
    // SkMatrixConvolutionImageFilter_TileMode
    ("TileMode", rewrite::k_xxx_name),
    // SkCanvas_*
    ("PointMode", rewrite::k_xxx_name),
    ("SrcRectConstraint", rewrite::k_xxx_name),
    // SkCanvas_Lattice_RectType
    ("RectType", rewrite::k_xxx),
    // SkDisplacementMapEffect_ChannelSelectorType
    ("ChannelSelectorType", rewrite::k_xxx_name),
    // SkDropShadowImageFilter_ShadowMode
    ("ShadowMode", rewrite::k_xxx_name),
    // SkFont_Edging
    ("Edging", rewrite::k_xxx),
    // SkFont_Slant
    ("Slant", rewrite::k_xxx_name),
    // SkHighContrastConfig_InvertStyle
    ("InvertStyle", rewrite::k_xxx),
    // SkImage_*
    ("BitDepth", rewrite::k_xxx),
    ("CachingHint", rewrite::k_xxx_name),
    ("CompressionType", rewrite::k_xxx),
    // SkImageFilter_MapDirection
    ("MapDirection", rewrite::k_xxx_name),
    // SkCodec_Result
    // SkInterpolatorBase_Result
    ("Result", rewrite::k_xxx),
    // SkMatrix_ScaleToFit
    ("ScaleToFit", rewrite::k_xxx_name),
    // SkPath_*
    ("ArcSize", rewrite::k_xxx_name),
    ("AddPathMode", rewrite::k_xxx_name),
    // SkRegion_Op
    // TODO: remove kLastOp?
    ("Op", rewrite::k_xxx_name_opt),
    // SkRRect_*
    // TODO: remove kLastType?
    // SkRuntimeEffect_Variable_Type
    ("Type", rewrite::k_xxx_name_opt),
    ("Corner", rewrite::k_xxx_name),
    // SkShader_GradientType
    ("GradientType", rewrite::k_xxx_name),
    // SkSurface_*
    ("ContentChangeMode", rewrite::k_xxx_name),
    ("BackendHandleAccess", rewrite::k_xxx_name),
    // SkTextUtils_Align
    ("Align", rewrite::k_xxx_name),
    // SkTrimPathEffect_Mode
    ("Mode", rewrite::k_xxx),
    // SkTypeface_SerializeBehavior
    ("SerializeBehavior", rewrite::k_xxx),
    // SkVertices_VertexMode
    ("VertexMode", rewrite::k_xxx_name),
    // SkYUVAIndex_Index
    ("Index", rewrite::k_xxx_name),
    // SkRuntimeEffect_Variable_Qualifier
    ("Qualifier", rewrite::k_xxx),
    // private type that leaks through SkRuntimeEffect_Variable
    ("GrSLType", rewrite::k_xxx_name),
    //
    // gpu/
    //
    ("GrGLStandard", rewrite::k_xxx_name),
    ("GrGLFormat", rewrite::k_xxx),
    ("GrSurfaceOrigin", rewrite::k_xxx_name),
    ("GrBackendApi", rewrite::k_xxx),
    ("GrMipMapped", rewrite::k_xxx),
    ("GrRenderable", rewrite::k_xxx),
    ("GrProtected", rewrite::k_xxx),
    //
    // DartTypes.h
    //
    ("Affinity", rewrite::k_xxx),
    ("RectHeightStyle", rewrite::k_xxx),
    ("RectWidthStyle", rewrite::k_xxx),
    ("TextAlign", rewrite::k_xxx),
    ("TextDirection", rewrite::k_xxx_uppercase),
    ("TextBaseline", rewrite::k_xxx),
    ("TextHeightBehavior", rewrite::k_xxx),
    //
    // TextStyle.h
    //
    ("TextDecorationStyle", rewrite::k_xxx),
    ("TextDecorationMode", rewrite::k_xxx),
    ("StyleType", rewrite::k_xxx),
    ("PlaceholderAlignment", rewrite::k_xxx),
    //
    // Vk*
    //
    ("VkChromaLocation", rewrite::vk),
    ("VkFilter", rewrite::vk),
    ("VkFormat", rewrite::vk),
    ("VkImageLayout", rewrite::vk),
    ("VkImageTiling", rewrite::vk),
    ("VkSamplerYcbcrModelConversion", rewrite::vk),
    ("VkSamplerYcbcrRange", rewrite::vk),
    ("VkStructureType", rewrite::vk),
    // m84: SkPath::Verb
    ("Verb", rewrite::k_xxx_name),
    // m84: SkVertices::Attribute::Usage
    ("Usage", rewrite::k_xxx),
    ("GrSemaphoresSubmitted", rewrite::k_xxx),
    ("BackendSurfaceAccess", rewrite::k_xxx),
    // m85: VkSharingMode
    ("VkSharingMode", rewrite::vk),
];

pub(crate) mod rewrite {
    use heck::ShoutySnakeCase;
    use regex::Regex;

    pub fn k_xxx_uppercase(name: &str, variant: &str) -> String {
        k_xxx(name, variant).to_uppercase()
    }

    pub fn k_xxx(name: &str, variant: &str) -> String {
        if variant.starts_with('k') {
            variant[1..].into()
        } else {
            panic!(
                "Variant name '{}' of enum type '{}' is expected to start with a 'k'",
                variant, name
            );
        }
    }

    pub fn _k_xxx_enum(name: &str, variant: &str) -> String {
        capture(name, variant, &format!("k(.*)_{}", name))
    }

    pub fn k_xxx_name_opt(name: &str, variant: &str) -> String {
        let suffix = &format!("_{}", name);
        if variant.ends_with(suffix) {
            capture(name, variant, &format!("k(.*){}", suffix))
        } else {
            capture(name, variant, "k(.*)")
        }
    }

    pub fn k_xxx_name(name: &str, variant: &str) -> String {
        capture(name, variant, &format!("k(.*)_{}", name))
    }

    pub fn vk(name: &str, variant: &str) -> String {
        let prefix = name.to_shouty_snake_case();
        capture(name, variant, &format!("{}_(.*)", prefix))
    }

    fn capture(name: &str, variant: &str, pattern: &str) -> String {
        let re = Regex::new(pattern).unwrap();
        re.captures(variant).unwrap_or_else(|| {
            panic!(
                "failed to match '{}' on enum variant '{}' of enum '{}'",
                pattern, variant, name
            )
        })[1]
            .into()
    }
}

mod prerequisites {
    use crate::build_support::{cargo, utils};
    use flate2::read::GzDecoder;
    use std::ffi::OsStr;
    use std::fs;
    use std::io::Cursor;
    use std::path::Component;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    pub fn locate_python2_cmd() -> PathBuf {
        const PYTHON_CMDS: [&str; 2] = ["python", "python2"];
        for python in PYTHON_CMDS.as_ref() {
            println!("Probing '{}'", python);
            if let Some(true) = is_python_version_2(python) {
                return python.into();
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
            let archive = utils::download(archive_url)
                .unwrap_or_else(|err| panic!("Failed to download {} ({})", archive_url, err));

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
                url: "https://codeload.github.com/rust-skia/skia/tar.gz",
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
            !matches!(p.components().next(),
                Some(Component::Normal(name)) if name == OsStr::new("infra"))
        }

        // we need only ninja from depot_tools.
        // https://github.com/rust-skia/rust-skia/pull/165
        fn filter_depot_tools(p: &Path) -> bool {
            p.to_str().unwrap().starts_with("ninja")
        }
    }
}

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
                let items: Vec<&str> = d.splitn(2, '=').collect();
                match items.len() {
                    1 => (items[0].to_string(), None),
                    2 => (items[0].to_string(), Some(items[1].to_string())),
                    _ => panic!("internal error"),
                }
            })
            .collect()
    }

    pub fn combine(a: Definitions, b: Definitions) -> Definitions {
        remove_duplicates(a.into_iter().chain(b.into_iter()).collect())
    }

    pub fn remove_duplicates(mut definitions: Definitions) -> Definitions {
        let mut uniques = HashSet::new();
        definitions.retain(|e| uniques.insert(e.0.clone()));
        definitions
    }
}

mod ninja {
    use std::path::PathBuf;

    pub fn default_exe_name() -> PathBuf {
        if cfg!(windows) { "ninja.exe" } else { "ninja" }.into()
    }
}
