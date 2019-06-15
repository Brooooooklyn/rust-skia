use crate::prelude::*;
use skia_bindings::{
    C_SkFontStyle_Construct, C_SkFontStyle_Equals, SkFontStyle, SkFontStyle_Slant,
    SkFontStyle_Weight, SkFontStyle_Width,
};
use std::mem;

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Weight(i32);

impl NativeTransmutable<i32> for Weight {}

#[test]
fn test_weight_layout() {
    Weight::test_layout()
}

#[allow(non_upper_case_globals)]
impl Weight {
    pub const Invisible: Self = Self(SkFontStyle_Weight::kInvisible_Weight as _);
    pub const Thin: Self = Self(SkFontStyle_Weight::kThin_Weight as _);
    pub const ExtraLight: Self = Self(SkFontStyle_Weight::kExtraLight_Weight as _);
    pub const Light: Self = Self(SkFontStyle_Weight::kLight_Weight as _);
    pub const Normal: Self = Self(SkFontStyle_Weight::kNormal_Weight as _);
    pub const Medium: Self = Self(SkFontStyle_Weight::kMedium_Weight as _);
    pub const SemiBold: Self = Self(SkFontStyle_Weight::kSemiBold_Weight as _);
    pub const Bold: Self = Self(SkFontStyle_Weight::kBold_Weight as _);
    pub const ExtraBold: Self = Self(SkFontStyle_Weight::kExtraBold_Weight as _);
    pub const Black: Self = Self(SkFontStyle_Weight::kBlack_Weight as _);
    pub const ExtraBlack: Self = Self(SkFontStyle_Weight::kExtraBlack_Weight as _);
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Width(i32);

impl NativeTransmutable<i32> for Width {}

#[test]
fn test_width_layout() {
    Width::test_layout()
}

#[allow(non_upper_case_globals)]
impl Width {
    pub const UltraCondensed: Self = Self(SkFontStyle_Width::kUltraCondensed_Width as _);
    pub const ExtraCondensed: Self = Self(SkFontStyle_Width::kExtraCondensed_Width as _);
    pub const Condensed: Self = Self(SkFontStyle_Width::kCondensed_Width as _);
    pub const SemiCondensed: Self = Self(SkFontStyle_Width::kSemiCondensed_Width as _);
    pub const Normal: Self = Self(SkFontStyle_Width::kNormal_Width as _);
    pub const SemiExpanded: Self = Self(SkFontStyle_Width::kSemiExpanded_Width as _);
    pub const Expanded: Self = Self(SkFontStyle_Width::kExpanded_Width as _);
    pub const ExtraExpanded: Self = Self(SkFontStyle_Width::kExtraExpanded_Width as _);
    pub const UltraExpanded: Self = Self(SkFontStyle_Width::kUltraExpanded_Width as _);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum Slant {
    Upright = SkFontStyle_Slant::kUpright_Slant as _,
    Italic = SkFontStyle_Slant::kItalic_Slant as _,
    Oblique = SkFontStyle_Slant::kOblique_Slant as _,
}

impl NativeTransmutable<SkFontStyle_Slant> for Slant {}

#[test]
fn test_slant_layout() {
    Slant::test_layout()
}

// TODO: implement Display
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct FontStyle(SkFontStyle);

impl NativeTransmutable<SkFontStyle> for FontStyle {}
#[test]
fn test_font_style_layout() {
    FontStyle::test_layout()
}

impl PartialEq for FontStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { C_SkFontStyle_Equals(self.native(), rhs.native()) }
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        // does not link under Linux:
        // unsafe { SkFontStyle::new1() }
        FontStyle::from_native(unsafe {
            let mut font_style = mem::uninitialized();
            C_SkFontStyle_Construct(&mut font_style);
            font_style
        })
    }
}

impl FontStyle {
    pub fn new(weight: Weight, width: Width, slant: Slant) -> Self {
        Self::from_native(unsafe {
            SkFontStyle::new(*weight.native(), *width.native(), *slant.native())
        })
    }

    pub fn weight(self) -> Weight {
        Weight::from_native(unsafe { self.native().weight() })
    }

    pub fn width(self) -> Width {
        Width::from_native(unsafe { self.native().width() })
    }

    pub fn slant(self) -> Slant {
        Slant::from_native(unsafe { self.native().slant() })
    }

    pub fn normal() -> FontStyle {
        *font_style_static::NORMAL
    }

    pub fn bold() -> FontStyle {
        *font_style_static::BOLD
    }

    pub fn italic() -> FontStyle {
        *font_style_static::ITALIC
    }

    pub fn bold_italic() -> FontStyle {
        *font_style_static::BOLD_ITALIC
    }
}

mod font_style_static {
    use super::{FontStyle, Slant, Weight, Width};

    lazy_static! {
        pub static ref NORMAL: FontStyle =
            FontStyle::new(Weight::Normal, Width::Normal, Slant::Upright);
        pub static ref BOLD: FontStyle =
            FontStyle::new(Weight::Bold, Width::Normal, Slant::Upright);
        pub static ref ITALIC: FontStyle =
            FontStyle::new(Weight::Normal, Width::Normal, Slant::Italic);
        pub static ref BOLD_ITALIC: FontStyle =
            FontStyle::new(Weight::Bold, Width::Normal, Slant::Italic);
    }
}

#[test]
fn test_equality() {
    let style: FontStyle = Default::default();
    let style2: FontStyle = Default::default();
    assert!(style == style2);
}
