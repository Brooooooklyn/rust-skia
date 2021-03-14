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
    pub const SK_SHAPER: &str = "skshaper";
    pub const SK_PARAGRAPH: &str = "skparagraph";
}

/// Feature identifiers define the additional configuration parts of the binaries to download.
mod feature_id {
    pub const GL: &str = "gl";
    pub const VULKAN: &str = "vulkan";
    pub const METAL: &str = "metal";
    pub const D3D: &str = "d3d";
    pub const TEXTLAYOUT: &str = "textlayout";
    pub const WEBPE: &str = "webpe";
    pub const WEBPD: &str = "webpd";
    pub const EGL: &str = "egl";
    pub const X11: &str = "x11";
    pub const WAYLAND: &str = "wayland";
}

/// The defaults for the Skia build configuration.
impl Default for BuildConfiguration {
    fn default() -> Self {
        let skia_debug = matches!(cargo::env_var("SKIA_DEBUG"), Some(v) if v != "0");

        BuildConfiguration {
            on_windows: cargo::host().is_windows(),
            skia_debug,
            // `OPT_LEVEL` is set by Cargo itself.
            opt_level: cargo::env_var("OPT_LEVEL"),
            features: Features {
                gl: cfg!(feature = "gl"),
                egl: cfg!(feature = "egl"),
                wayland: cfg!(feature = "wayland"),
                x11: cfg!(feature = "x11"),
                vulkan: cfg!(feature = "vulkan"),
                metal: cfg!(feature = "metal"),
                d3d: cfg!(feature = "d3d"),
                text_layout: cfg!(feature = "textlayout"),
                webp_encode: cfg!(feature = "webp-encode"),
                webp_decode: cfg!(feature = "webp-decode"),
                animation: false,
                dng: false,
                particles: false,
            },
            definitions: Vec::new(),
            cc: cargo::env_var("CC").unwrap_or_else(|| "clang".to_string()),
            cxx: cargo::env_var("CXX").unwrap_or_else(|| "clang++".to_string()),
        }
    }
}

/// The build configuration for Skia.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildConfiguration {
    /// Do we build _on_ a Windows OS?
    on_windows: bool,

    /// Set the optimization level (0-3, s or z). Clang and GCC use the same notation
    /// as Rust, so we just pass this option through from Cargo.
    opt_level: Option<String>,

    /// Build Skia in a debug configuration?
    skia_debug: bool,

    /// The Skia feature set to compile.
    features: Features,

    /// Additional preprocessor definitions that will override predefined ones.
    definitions: Definitions,

    /// C compiler to use
    cc: String,

    /// C++ compiler to use
    cxx: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Features {
    /// Build with OpenGL support?
    pub gl: bool,

    /// Build with EGL support? If you set X11, setting this to false will use LibGL (GLX)
    pub egl: bool,

    /// Build with Wayland support? This requires EGL, as GLX does not work on Wayland.
    pub wayland: bool,

    /// Build with X11 support?
    pub x11: bool,

    /// Build with Vulkan support?
    pub vulkan: bool,

    /// Build with Metal support?
    pub metal: bool,

    /// Build with Direct3D support?
    pub d3d: bool,

    /// Features related to text layout. Modules skshaper and skparagraph.
    pub text_layout: bool,

    /// Support the encoding of bitmap data to the WEBP image format.
    pub webp_encode: bool,

    /// Support the decoding of the WEBP image format to bitmap data.
    pub webp_decode: bool,

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
        if self.egl {
            feature_ids.push(feature_id::EGL);
        }
        if self.x11 {
            feature_ids.push(feature_id::X11);
        }
        if self.wayland {
            feature_ids.push(feature_id::WAYLAND);
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
        if self.webp_encode {
            feature_ids.push(feature_id::WEBPE);
        }
        if self.webp_decode {
            feature_ids.push(feature_id::WEBPD);
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

    /// Whether to use system libraries or not.
    pub use_system_libraries: bool,
}

impl FinalBuildConfiguration {
    pub fn from_build_configuration(
        build: &BuildConfiguration,
        use_system_libraries: bool,
        skia_source_dir: &Path,
    ) -> FinalBuildConfiguration {
        let features = &build.features;

        let gn_args = {
            fn quote(s: &str) -> String {
                format!("\"{}\"", s)
            }

            let mut args: Vec<(&str, String)> = vec![
                ("is_official_build", yes_if(!build.skia_debug)),
                ("is_debug", yes_if(build.skia_debug)),
                ("skia_enable_gpu", yes_if(features.gpu())),
                ("skia_use_gl", yes_if(features.gl)),
                ("skia_use_egl", yes_if(features.egl)),
                ("skia_use_x11", yes_if(features.x11)),
                (
                    "skia_use_system_libjpeg_turbo",
                    yes_if(use_system_libraries),
                ),
                ("skia_use_system_libpng", yes_if(use_system_libraries)),
                ("skia_use_libwebp_encode", yes_if(features.webp_encode)),
                ("skia_use_libwebp_decode", yes_if(features.webp_decode)),
                ("skia_use_system_zlib", yes_if(use_system_libraries)),
                ("skia_use_xps", no()),
                ("skia_use_dng_sdk", yes_if(features.dng)),
                ("cc", quote(&build.cc)),
                ("cxx", quote(&build.cxx)),
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
                    ("skia_use_system_icu", yes_if(use_system_libraries)),
                    ("skia_use_harfbuzz", yes()),
                    ("skia_pdf_subset_harfbuzz", yes()),
                    ("skia_use_system_harfbuzz", yes_if(use_system_libraries)),
                    ("skia_use_sfntly", no()),
                    ("skia_enable_skparagraph", yes()),
                    // note: currently, tests need to be enabled, because modules/skparagraph
                    // is not included in the default dependency configuration.
                    // ("paragraph_tests_enabled", no()),
                ]);
            } else {
                args.push(("skia_use_icu", no()));
            }

            if features.webp_encode || features.webp_decode {
                args.push(("skia_use_system_libwebp", yes_if(use_system_libraries)))
            }

            let mut use_expat = true;

            // target specific gn args.
            let target = cargo::target();
            let target_str: &str = &format!("--target={}", target.to_string());
            let sysroot_arg;
            let opt_level_arg;
            let mut cflags: Vec<&str> = vec![&target_str];
            let asmflags: Vec<&str> = vec![&target_str];

            if let Some(sysroot) = cargo::env_var("SDKROOT") {
                sysroot_arg = format!("--sysroot={}", sysroot);
                cflags.push(&sysroot_arg);
            }

            if let Some(opt_level) = &build.opt_level {
                /* LTO generates corrupt libraries on the host platforms when building with --release
                if opt_level.parse::<usize>() != Ok(0) {
                    cflags.push("-flto");
                }
                */
                // When targeting windows `-O` isn't supported.
                if !target.is_windows() {
                    opt_level_arg = format!("-O{}", opt_level);
                    cflags.push(&opt_level_arg);
                }
            }

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
                        cflags.push("/MT");
                    } else {
                        // otherwise the C runtime should be linked dynamically
                        cflags.push("/MD");
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
                    args.push(("skia_use_system_freetype2", yes_if(use_system_libraries)));
                    args.push(("skia_enable_fontmgr_android", yes()));
                    // Enabling fontmgr_android implicitly enables expat.
                    // We make this explicit to avoid relying on an expat installed
                    // in the system.
                    use_expat = true;
                }
                (arch, _, "ios", _) => {
                    args.push(("target_os", quote("ios")));
                    args.push(("target_cpu", quote(clang::target_arch(arch))));
                    ios::extra_skia_cflags(arch, &mut cflags);
                }
                (arch, _, os, _) => {
                    let skia_target_os = match os {
                        "darwin" => "mac",
                        "windows" => "win",
                        _ => os,
                    };
                    args.push(("target_os", quote(skia_target_os)));
                    args.push(("target_cpu", quote(clang::target_arch(arch))));
                }
            }

            if use_expat {
                args.push(("skia_use_expat", yes()));
                args.push(("skia_use_system_expat", yes_if(use_system_libraries)));
            } else {
                args.push(("skia_use_expat", no()));
            }

            if !cflags.is_empty() {
                let cflags = format!(
                    "[{}]",
                    cflags.into_iter().map(quote).collect::<Vec<_>>().join(",")
                );
                args.push(("extra_cflags", cflags));
            }

            if !asmflags.is_empty() {
                let asmflags = format!(
                    "[{}]",
                    asmflags
                        .into_iter()
                        .map(quote)
                        .collect::<Vec<_>>()
                        .join(",")
                );
                args.push(("extra_asmflags", asmflags));
            }

            args.into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        };

        let ninja_files = {
            let mut files = vec!["obj/skia.ninja".into()];
            if features.text_layout {
                files.extend(vec![
                    "obj/modules/skshaper/skshaper.ninja".into(),
                    "obj/modules/skparagraph/skparagraph.ninja".into(),
                ]);
            }
            files
        };

        let binding_sources = {
            let mut sources: Vec<PathBuf> = vec!["src/bindings.cpp".into()];
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
            use_system_libraries,
        }
    }
}

fn yes() -> String {
    "true".into()
}
fn no() -> String {
    "false".into()
}
fn yes_if(y: bool) -> String {
    if y {
        yes()
    } else {
        no()
    }
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

impl BinariesConfiguration {
    /// Build a binaries configuration based on the current environment cargo
    /// supplies us with and a Skia build configuration.
    pub fn from_cargo_env(build: &BuildConfiguration) -> Self {
        let features = &build.features;
        let target = cargo::target();

        let mut built_libraries = Vec::new();
        let feature_ids = features.ids();

        if features.text_layout {
            built_libraries.push(lib::SK_PARAGRAPH.into());
            built_libraries.push(lib::SK_SHAPER.into());
        }

        let mut link_libraries = Vec::new();

        match target.as_strs() {
            (_, "unknown", "linux", _) => {
                link_libraries.extend(vec!["stdc++", "fontconfig", "freetype"]);
                if features.gl {
                    if features.egl {
                        link_libraries.push("EGL");
                    }

                    if features.x11 {
                        link_libraries.push("GL");
                    }

                    if features.wayland {
                        link_libraries.push("wayland-egl");
                        link_libraries.push("GLESv2");
                    }
                }
            }
            (_, "apple", "darwin", _) => {
                link_libraries.extend(vec!["c++", "framework=ApplicationServices"]);
                if features.gl {
                    link_libraries.push("framework=OpenGL");
                }
                if features.metal {
                    link_libraries.push("framework=Metal");
                    // MetalKit was added in m87 BUILD.gn.
                    link_libraries.push("framework=MetalKit");
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
            additional_files: Vec::new(),
            skia_debug: build.skia_debug,
        }
    }

    /// Inform cargo that the library files of the given configuration are available and
    /// can be used as dependencies.
    pub fn commit_to_cargo(&self) {
        cargo::add_link_search(self.output_directory.to_str().unwrap());

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

/// Orchestrates the entire build of Skia based on the arguments provided.
pub fn build(
    build: &FinalBuildConfiguration,
    config: &BinariesConfiguration,
    ninja_command: Option<PathBuf>,
    gn_command: Option<PathBuf>,
    offline: bool,
) {
    let python2 = &prerequisites::locate_python2_cmd();
    println!("Python 2 found: {:?}", python2);

    let ninja = ninja_command.unwrap_or_else(|| {
        env::current_dir()
            .unwrap()
            .join("depot_tools")
            .join(ninja::default_exe_name())
    });

    if !offline && !build.use_system_libraries {
        println!("Synchronizing Skia dependencies");
        prerequisites::resolve_dependencies();
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
    }

    configure_skia(build, config, &python2, gn_command.as_deref());
    build_skia(build, config, &ninja);
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
    let builder = bindgen::Builder::default()
        .generate_comments(false)
        .layout_tests(true)
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
        .raw_line("pub enum GrRecordingContextPriv {}")
        .blacklist_type("GrRecordingContextPriv")
        .raw_line("pub enum GrContextPriv {}")
        .blacklist_type("GrContextPriv")
        .blacklist_function("GrContext_priv.*")
        .blacklist_function("SkDeferredDisplayList_priv.*")
        .raw_line("pub enum SkVerticesPriv {}")
        .blacklist_type("SkVerticesPriv")
        .blacklist_function("SkVertices_priv.*")
        .blacklist_function("std::bitset_flip.*")
        // Vulkan reexports that got swallowed by making them opaque.
        // (these can not be whitelisted by a extern "C" function)
        .whitelist_type("VkPhysicalDeviceFeatures")
        .whitelist_type("VkPhysicalDeviceFeatures2")
        // misc
        .whitelist_var("SK_Color.*")
        .whitelist_var("kAll_GrBackendState")
        .use_core()
        .clang_arg("-std=c++17")
        .clang_args(&["-x", "c++"])
        .clang_arg("-v");

    // don't generate destructors on Windows: https://github.com/rust-skia/rust-skia/issues/318
    let mut builder = if cfg!(target_os = "windows") {
        builder.with_codegen_config({
            let mut config = CodegenConfig::default();
            config.remove(CodegenConfig::DESTRUCTORS);
            config
        })
    } else {
        builder
    };

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

    // Whether GIF decoding is supported,
    // is decided by BUILD.gn based on the existence of the libgifcodec directory:
    if !definitions.iter().any(|(v, _)| v == "SK_USE_LIBGIFCODEC") {
        cargo::warning("GIF decoding support may be missing, does the directory skia/third_party/externals/libgifcodec/ exist?")
    }

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

    let target_str = &target.to_string();
    cc_build.target(target_str);

    let sdk;
    let sysroot = cargo::env_var("SDKROOT");
    let mut sysroot: Option<&str> = sysroot.as_ref().map(AsRef::as_ref);
    let mut sysroot_flag = "--sysroot=";

    match target.as_strs() {
        (_, "apple", "darwin", _) => {
            // macOS uses `-isysroot/path/to/sysroot`, but this doesn't appear
            // to work for other targets. `--sysroot=` works for all targets,
            // to my knowledge, but doesn't seem to be idiomatic for macOS
            // compilation. To capture this, we allow manually setting sysroot
            // on any platform, but we use `-isysroot` for OSX builds and `--sysroot`
            // elsewhere. If you don't manually set the sysroot, we can automatically
            // detect it, but this is only possible for macOS.
            sysroot_flag = "-isysroot";

            if sysroot.is_none() {
                if let Some(macos_sdk) = xcode::get_sdk_path("macosx") {
                    sdk = macos_sdk;
                    sysroot = Some(
                        sdk.to_str()
                            .expect("macOS SDK path could not be converted to string"),
                    );
                } else {
                    cargo::warning("failed to get macosx SDK path")
                }
            }
        }
        (arch, "linux", "android", _) | (arch, "linux", "androideabi", _) => {
            for arg in android::additional_clang_args(target_str, arch) {
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

    if let Some(sysroot) = sysroot {
        let sysroot = format!("{}{}", sysroot_flag, sysroot);
        builder = builder.clang_arg(&sysroot);
        cc_build.flag(&sysroot);
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
    // m86:
    "GrRecordingContext",
    "GrDirectContext",
    // m87:
    "GrD3DAlloc",
    "GrD3DMemoryAllocator",
    // m87, yuva_pixmaps
    "std::tuple",
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
    ("GrMipmapped", rewrite::k_xxx),
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
    ("DrawOptions", rewrite::k_xxx),
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
    // m85
    ("VkSharingMode", rewrite::vk),
    // m86:
    ("SkFilterMode", rewrite::k_xxx),
    ("SkMipmapMode", rewrite::k_xxx),
    ("Enable", rewrite::k_xxx),
    ("ShaderCacheStrategy", rewrite::k_xxx),
    // m87:
    // SkYUVAInfo_PlanarConfig
    ("PlanarConfig", rewrite::k_xxx),
    ("Siting", rewrite::k_xxx),
    // SkYUVAPixmapInfo
    ("DataType", rewrite::k_xxx),
    // m88:
    // SkYUVAInfo_*
    ("PlaneConfig", rewrite::k_xxx),
    // m89, SkImageFilters::Dither
    ("Dither", rewrite::k_xxx),
];

pub(crate) mod rewrite {
    use heck::ShoutySnakeCase;
    use regex::Regex;

    pub fn k_xxx_uppercase(name: &str, variant: &str) -> String {
        k_xxx(name, variant).to_uppercase()
    }

    pub fn k_xxx(name: &str, variant: &str) -> String {
        if let Some(stripped) = variant.strip_prefix('k') {
            stripped.into()
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

    /// Resolves the full path
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
                    if let Some(stripped) = d.strip_prefix(prefix) {
                        stripped
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
