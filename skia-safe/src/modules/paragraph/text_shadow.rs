use crate::{prelude::*, Color, Point};
use skia_bindings as sb;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TextShadow {
    pub color: Color,
    pub offset: Point,
    pub blur_sigma: f64,
}

impl NativeTransmutable<sb::skia_textlayout_TextShadow> for TextShadow {}

#[test]
fn text_shadow_layout() {
    TextShadow::test_layout()
}

impl Default for TextShadow {
    fn default() -> Self {
        TextShadow::from_native_c(unsafe { sb::skia_textlayout_TextShadow::new() })
    }
}

impl PartialEq for TextShadow {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_TextShadow_Equals(self.native(), other.native()) }
    }
}

impl TextShadow {
    pub fn new(color: impl Into<Color>, offset: impl Into<Point>, blur_sigma: f64) -> Self {
        TextShadow::from_native_c(unsafe {
            sb::skia_textlayout_TextShadow::new1(
                color.into().into_native(),
                offset.into().into_native(),
                blur_sigma,
            )
        })
    }

    pub fn has_shadow(&self) -> bool {
        unsafe { self.native().hasShadow() }
    }
}
