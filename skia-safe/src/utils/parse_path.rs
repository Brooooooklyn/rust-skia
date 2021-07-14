use crate::{
    interop,
    prelude::{IfBoolSome, NativeAccess},
    Path,
};
use skia_bindings as sb;
use std::ffi::CString;

pub fn from_svg(svg: impl AsRef<str>) -> Option<Path> {
    let str = CString::new(svg.as_ref()).unwrap();
    let mut path = Path::default();
    unsafe { sb::SkParsePath_FromSVGString(str.as_ptr(), path.native_mut()) }.if_true_some(path)
}

pub use skia_bindings::SkParsePath_PathEncoding as PathEncoding;

impl Path {
    pub fn from_svg(svg: impl AsRef<str>) -> Option<Path> {
        from_svg(svg)
    }

    pub fn to_svg(&self) -> String {
        to_svg(self)
    }

    pub fn to_svg_with_encoding(&self, encoding: PathEncoding) -> String {
        to_svg_with_encoding(self, encoding)
    }
}

pub fn to_svg(path: &Path) -> String {
    to_svg_with_encoding(path, PathEncoding::Absolute)
}

pub fn to_svg_with_encoding(path: &Path, encoding: PathEncoding) -> String {
    let mut svg = interop::String::default();
    unsafe { sb::SkParsePath_ToSVGString(path.native(), svg.native_mut(), encoding) };
    svg.as_str().into()
}

#[cfg(test)]
mod tests {
    use super::PathEncoding;

    #[test]
    fn test_path_encoding_naming() {
        let _ = PathEncoding::Absolute;
    }
}
