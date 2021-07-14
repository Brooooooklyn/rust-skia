use super::{FontFamilies, TextBaseline, TextShadow};
use crate::{
    interop::{self, AsStr, FromStrs, SetStr},
    prelude::*,
    scalar,
    textlayout::{RangeExtensions, EMPTY_INDEX, EMPTY_RANGE},
    Color, FontMetrics, FontStyle, Paint, Typeface,
};
use skia_bindings as sb;
use std::{fmt, ops::Range};

bitflags! {
    pub struct TextDecoration: u32 {
        const NO_DECORATION = sb::skia_textlayout_TextDecoration::kNoDecoration as _;
        const UNDERLINE = sb::skia_textlayout_TextDecoration::kUnderline as _;
        const OVERLINE = sb::skia_textlayout_TextDecoration::kOverline as _;
        const LINE_THROUGH = sb::skia_textlayout_TextDecoration::kLineThrough as _;
    }
}

pub const ALL_TEXT_DECORATIONS: TextDecoration = TextDecoration::ALL;

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration::NO_DECORATION
    }
}

impl TextDecoration {
    pub const ALL: TextDecoration = TextDecoration::all();
}

pub use sb::skia_textlayout_TextDecorationStyle as TextDecorationStyle;
#[test]
fn text_decoration_style_naming() {
    let _ = TextDecorationStyle::Solid;
}

pub use sb::skia_textlayout_TextDecorationMode as TextDecorationMode;
#[test]
fn text_decoration_mode_naming() {
    let _ = TextDecorationMode::Gaps;
}

pub use sb::skia_textlayout_StyleType as StyleType;
#[test]
fn style_type_member_naming() {
    let _ = StyleType::Foreground;
    let _ = StyleType::LetterSpacing;
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Decoration {
    pub ty: TextDecoration,
    pub mode: TextDecorationMode,
    pub color: Color,
    pub style: TextDecorationStyle,
    pub thickness_multiplier: scalar,
}

impl NativeTransmutable<sb::skia_textlayout_Decoration> for Decoration {}

#[test]
fn decoration_layout() {
    Decoration::test_layout();
}

pub use sb::skia_textlayout_PlaceholderAlignment as PlaceholderAlignment;

#[test]
fn placeholder_alignment_member_naming() {
    let _ = PlaceholderAlignment::Baseline;
    let _ = PlaceholderAlignment::AboveBaseline;
}

pub type FontFeature = Handle<sb::skia_textlayout_FontFeature>;
unsafe impl Send for FontFeature {}
unsafe impl Sync for FontFeature {}

impl NativeDrop for sb::skia_textlayout_FontFeature {
    fn drop(&mut self) {
        unsafe { sb::C_FontFeature_destruct(self) }
    }
}

impl NativeClone for sb::skia_textlayout_FontFeature {
    fn clone(&self) -> Self {
        construct(|ts| unsafe { sb::C_FontFeature_CopyConstruct(ts, self) })
    }
}

impl PartialEq for FontFeature {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.value() == other.value()
    }
}

impl fmt::Debug for FontFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FontFeature")
            .field(&self.name())
            .field(&self.value())
            .finish()
    }
}

impl FontFeature {
    pub fn name(&self) -> &str {
        self.native().fName.as_str()
    }

    pub fn value(&self) -> i32 {
        self.native().fValue
    }
}

#[repr(C)]
#[derive(Clone, Default, Debug)]
pub struct PlaceholderStyle {
    pub width: scalar,
    pub height: scalar,
    pub alignment: PlaceholderAlignment,
    pub baseline: TextBaseline,
    pub baseline_offset: scalar,
}

impl NativeTransmutable<sb::skia_textlayout_PlaceholderStyle> for PlaceholderStyle {}
#[test]
fn placeholder_style_layout() {
    PlaceholderStyle::test_layout()
}

impl PartialEq for PlaceholderStyle {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.native().equals(other.native()) }
    }
}

impl PlaceholderStyle {
    pub fn new(
        width: scalar,
        height: scalar,
        alignment: PlaceholderAlignment,
        baseline: TextBaseline,
        offset: scalar,
    ) -> Self {
        Self {
            width,
            height,
            alignment,
            baseline,
            baseline_offset: offset,
        }
    }
}

pub type TextStyle = Handle<sb::skia_textlayout_TextStyle>;
unsafe impl Send for TextStyle {}
unsafe impl Sync for TextStyle {}

impl NativeDrop for sb::skia_textlayout_TextStyle {
    fn drop(&mut self) {
        unsafe { sb::C_TextStyle_destruct(self) }
    }
}

impl NativeClone for sb::skia_textlayout_TextStyle {
    fn clone(&self) -> Self {
        construct(|ts| unsafe { sb::C_TextStyle_CopyConstruct(ts, self) })
    }
}

impl NativePartialEq for sb::skia_textlayout_TextStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { self.equals(rhs) }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextStyle")
            .field("color", &self.color())
            .field("foreground", &self.foreground())
            .field("background", &self.background())
            .field("decoration", &self.decoration())
            .field("font_style", &self.font_style())
            .field("shadows", &self.shadows())
            .field("font_features", &self.font_features())
            .field("font_size", &self.font_size())
            .field("font_families", &self.font_families())
            .field("height", &self.height())
            .field("height_override", &self.height_override())
            .field("half_leading", &self.half_leading())
            .field("letter_spacing", &self.letter_spacing())
            .field("word_spacing", &self.word_spacing())
            .field("typeface", &self.typeface())
            .field("locale", &self.locale())
            .field("text_baseline", &self.text_baseline())
            .field("is_placeholder", &self.is_placeholder())
            .finish()
    }
}

impl TextStyle {
    pub fn new() -> Self {
        TextStyle::construct(|ts| unsafe { sb::C_TextStyle_Construct(ts) })
    }

    pub fn to_placeholder(&self) -> Self {
        TextStyle::from_native_c(unsafe { sb::skia_textlayout_TextStyle::new(self.native(), true) })
    }

    pub fn equals(&self, other: &TextStyle) -> bool {
        *self == *other
    }

    pub fn equals_by_fonts(&self, that: &TextStyle) -> bool {
        unsafe { self.native().equalsByFonts(that.native()) }
    }

    pub fn match_one_attribute(&self, style_type: StyleType, other: &TextStyle) -> bool {
        unsafe { self.native().matchOneAttribute(style_type, other.native()) }
    }

    pub fn color(&self) -> Color {
        Color::from_native_c(self.native().fColor)
    }

    pub fn set_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.native_mut().fColor = color.into().into_native();
        self
    }

    pub fn foreground(&self) -> Option<&Paint> {
        self.native()
            .fHasForeground
            .if_true_some(Paint::from_native_ref(&self.native().fForeground))
    }

    pub fn set_foreground_color(&mut self, paint: impl Into<Option<Paint>>) -> &mut Self {
        let n = self.native_mut();
        n.fHasForeground = paint
            .into()
            .map(|paint| n.fForeground.replace_with(paint))
            .is_some();
        self
    }

    pub fn background(&self) -> Option<&Paint> {
        self.native()
            .fHasBackground
            .if_true_some(Paint::from_native_ref(&self.native().fBackground))
    }

    pub fn set_background_color(&mut self, paint: impl Into<Option<Paint>>) -> &mut Self {
        let n = self.native_mut();
        n.fHasBackground = paint
            .into()
            .map(|paint| n.fBackground.replace_with(paint))
            .is_some();
        self
    }

    pub fn decoration(&self) -> &Decoration {
        Decoration::from_native_ref(&self.native().fDecoration)
    }

    pub fn decoration_mut(&mut self) -> &mut Decoration {
        Decoration::from_native_ref_mut(&mut self.native_mut().fDecoration)
    }

    pub fn font_style(&self) -> FontStyle {
        FontStyle::from_native_c(self.native().fFontStyle)
    }

    pub fn set_font_style(&mut self, font_style: FontStyle) -> &mut Self {
        self.native_mut().fFontStyle = font_style.into_native();
        self
    }

    pub fn shadows(&self) -> &[TextShadow] {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getShadows(&self.native().fTextShadows, &mut count);
            safer::from_raw_parts(TextShadow::from_native_ptr(ptr), count)
        }
    }

    pub fn add_shadow(&mut self, shadow: TextShadow) -> &mut Self {
        unsafe { sb::C_TextStyle_addShadow(self.native_mut(), shadow.native()) }
        self
    }

    pub fn reset_shadows(&mut self) -> &mut Self {
        unsafe { sb::C_TextStyle_resetShadows(self.native_mut()) }
        self
    }

    pub fn font_features(&self) -> &[FontFeature] {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getFontFeatures(&self.native().fFontFeatures, &mut count);
            safer::from_raw_parts(FontFeature::from_native_ptr(ptr), count)
        }
    }

    pub fn add_font_feature(&mut self, font_feature: impl AsRef<str>, value: i32) {
        let font_feature = interop::String::from_str(font_feature);
        unsafe { sb::C_TextStyle_addFontFeature(self.native_mut(), font_feature.native(), value) }
    }

    pub fn reset_font_features(&mut self) {
        unsafe { sb::C_TextStyle_resetFontFeatures(self.native_mut()) }
    }

    pub fn font_size(&self) -> scalar {
        self.native().fFontSize
    }

    pub fn set_font_size(&mut self, size: scalar) -> &mut Self {
        self.native_mut().fFontSize = size;
        self
    }

    pub fn font_families(&self) -> FontFamilies {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getFontFamilies(self.native(), &mut count);
            FontFamilies(safer::from_raw_parts(ptr, count))
        }
    }

    pub fn set_font_families(&mut self, families: &[impl AsRef<str>]) -> &mut Self {
        let families: Vec<interop::String> = FromStrs::from_strs(families);
        let families = families.native();
        unsafe {
            sb::C_TextStyle_setFontFamilies(self.native_mut(), families.as_ptr(), families.len())
        }
        self
    }

    pub fn set_height(&mut self, height: scalar) -> &mut Self {
        self.native_mut().fHeight = height;
        self
    }

    pub fn height(&self) -> scalar {
        let n = self.native();
        if n.fHeightOverride {
            n.fHeight
        } else {
            0.0
        }
    }

    pub fn set_height_override(&mut self, height_override: bool) -> &mut Self {
        self.native_mut().fHeightOverride = height_override;
        self
    }

    pub fn height_override(&self) -> bool {
        self.native().fHeightOverride
    }

    pub fn set_half_leading(&mut self, half_leading: bool) -> &mut Self {
        self.native_mut().fHalfLeading = half_leading;
        self
    }

    pub fn half_leading(&self) -> bool {
        self.native().fHalfLeading
    }

    pub fn set_letter_spacing(&mut self, letter_spacing: scalar) -> &mut Self {
        self.native_mut().fLetterSpacing = letter_spacing;
        self
    }

    pub fn letter_spacing(&self) -> scalar {
        self.native().fLetterSpacing
    }

    pub fn set_word_spacing(&mut self, word_spacing: scalar) -> &mut Self {
        self.native_mut().fWordSpacing = word_spacing;
        self
    }

    pub fn word_spacing(&self) -> scalar {
        self.native().fWordSpacing
    }

    pub fn typeface(&self) -> Option<Typeface> {
        Typeface::from_unshared_ptr(self.native().fTypeface.fPtr)
    }

    pub fn set_typeface(&mut self, typeface: impl Into<Option<Typeface>>) -> &mut Self {
        unsafe {
            sb::C_TextStyle_setTypeface(self.native_mut(), typeface.into().into_ptr_or_null())
        }
        self
    }

    pub fn locale(&self) -> &str {
        self.native().fLocale.as_str()
    }

    pub fn set_locale(&mut self, locale: impl AsRef<str>) -> &mut Self {
        self.native_mut().fLocale.set_str(locale);
        self
    }

    pub fn text_baseline(&self) -> TextBaseline {
        self.native().fTextBaseline
    }

    pub fn set_text_baseline(&mut self, baseline: TextBaseline) -> &mut Self {
        self.native_mut().fTextBaseline = baseline;
        self
    }

    pub fn font_metrics(&self) -> FontMetrics {
        FontMetrics::construct(|fm| unsafe { self.native().getFontMetrics(fm) })
    }

    pub fn is_placeholder(&self) -> bool {
        self.native().fIsPlaceholder
    }

    pub fn set_placeholder(&mut self) -> &mut Self {
        self.native_mut().fIsPlaceholder = true;
        self
    }
}

pub type TextIndex = usize;
pub type TextRange = Range<usize>;
pub const EMPTY_TEXT: TextRange = EMPTY_RANGE;

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct Block {
    pub range: TextRange,
    pub style: TextStyle,
}

impl NativeTransmutable<sb::skia_textlayout_Block> for Block {}
#[test]
fn block_layout() {
    Block::test_layout()
}

impl Default for Block {
    fn default() -> Self {
        Self {
            range: EMPTY_RANGE,
            style: Default::default(),
        }
    }
}

impl Block {
    pub fn new(text_range: TextRange, style: TextStyle) -> Self {
        Self {
            range: text_range,
            style,
        }
    }

    pub fn add(&mut self, tail: TextRange) -> &mut Self {
        debug_assert!(self.range.end == tail.start);
        self.range = self.range.start..self.range.start + self.range.width() + tail.width();
        self
    }
}

pub type BlockIndex = usize;
pub type BlockRange = Range<usize>;

pub const EMPTY_BLOCK: usize = EMPTY_INDEX;
pub const EMPTY_BLOCKS: Range<usize> = EMPTY_RANGE;

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct Placeholder {
    pub range: TextRange,
    pub style: PlaceholderStyle,
    pub text_style: TextStyle,
    pub blocks_before: BlockRange,
    pub text_before: TextRange,
}

impl NativeTransmutable<sb::skia_textlayout_Placeholder> for Placeholder {}

#[test]
fn placeholder_layout() {
    Placeholder::test_layout()
}

impl Default for Placeholder {
    fn default() -> Self {
        #[allow(clippy::reversed_empty_ranges)]
        Self {
            range: EMPTY_RANGE,
            style: Default::default(),
            text_style: Default::default(),
            blocks_before: 0..0,
            text_before: 0..0,
        }
    }
}

impl Placeholder {
    pub fn new(
        range: Range<usize>,
        style: PlaceholderStyle,
        text_style: TextStyle,
        blocks_before: BlockRange,
        text_before: TextRange,
    ) -> Self {
        Self {
            range,
            style,
            text_style,
            blocks_before,
            text_before,
        }
    }
}
