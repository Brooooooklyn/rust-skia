Safe Rust bindings to the [Skia Graphics Library](https://skia.org/).

[![Build Status](https://dev.azure.com/pragmatrix-github/rust-skia/_apis/build/status/rust-skia.rust-skia?branchName=master)](https://dev.azure.com/pragmatrix-github/rust-skia/_build/latest?definitionId=2&branchName=master)

Skia Submodule Status: chrome/m73 ([pending changes][skiapending]).

[skiapending]: https://github.com/google/skia/compare/2c36ee834ae04d036363cd3b8f3f33ec65d657f0...chrome/m73

## Goals

This project attempts to provide safe bindings that bridge between Skia's C++ API and idiomatic Rust on all major desktop, mobile, and [WebAssembly](https://en.wikipedia.org/wiki/WebAssembly) platforms, including GPU rendering support for [Vulkan](https://en.wikipedia.org/wiki/Vulkan_(API)), [Metal](https://en.wikipedia.org/wiki/Metal_(API)), and [OpenGL](https://en.wikipedia.org/wiki/OpenGL).

## Building

`cargo build`

Just kidding, we wish it were that simple. Currently you need _at least_ to install LLVM, depot_tools, and OpenGL libraries. For some detailed information about how to install the prerequisites on your platform, take a look at the [template we use to build on Azure](https://github.com/rust-skia/rust-skia/blob/master/azure-pipelines-template.yml).

Please share your experience so that we can complete this section here and try to automate the build to get to the point where `cargo build` _is_ sufficient to build the bindings _including_ Skia, and if that is not possible, clearly prompts to what's missing.

To simplify and speed up the build, we plan to provide prebuilt binaries for some of the major platforms.

## Examples

The examples are taken from [Skia's website](https://skia.org/) and [ported to the Rust API](skia-safe/examples/skia-org).

If you were able to build the project, run

`cargo run --example skia-org [OUTPUT_DIR]` 

to generate some Skia drawn PNG images in the directory `OUTPUT_DIR`.

## Status

### Crate

Due to the size and it's build requirements of Skia, we'd like to experiment with prebuilt binaries before releasing a crate.

### Platforms

- [x] Windows
- [x] Linux Ubuntu 16 (18 should work, too).
- [x] MacOSX
- [ ] WebAssembly: [#42](https://github.com/rust-skia/rust-skia/pull/42).
- [ ] Android
- [ ] iOS

### Bindings

Skia is a large library. While we strife to bind all of the C++ APIs, it's nowhere complete yet. 

We do support most of the SkCanvas, SkPaint, and SkPath and related APIs and are trying to make the examples from the [skia.org](https://skia.org/) website work. Upcoming are the bindings for the classes in the [`include/effects/`](https://github.com/google/skia/tree/2c36ee834ae04d036363cd3b8f3f33ec65d657f0/include/effects) directory.

### Features

- [x] Vector Graphics: Matrix, Rect, Point, Size, etc.
- [x] Basic Drawing: Surface, Canvas, Paint, Path.
- [ ] Effects and Shaders [#45](https://github.com/rust-skia/rust-skia/pull/45)
- [ ] PDF
- [ ] SVG
- [ ] XPS
- [ ] Animation
- [x] Vulkan (rudimentary texture drawing support, enable with the cargo feature "vulkan").
- [ ] Metal
- [ ] OpenGL

## This project needs contributions!

If you'd like help with the bindings, take a look at the [Wiki](https://github.com/rust-skia/rust-skia/wiki) to get started and create an issue to avoid duplicate work. For smaller tasks, grep for "TODO" in the source code. And if you want to help making the Rust API nicer, look out for open issues with the label [api conventions](https://github.com/rust-skia/rust-skia/issues?q=is%3Aissue+is%3Aopen+label%3A%22api+conventions%22).

## Maintainers

- LongYinan (@Brooooooklyn)
- Armin (@pragmatrix)

## License

MIT

