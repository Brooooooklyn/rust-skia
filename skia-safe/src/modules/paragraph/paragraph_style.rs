use std::fmt;

use super::{DrawOptions, FontFamilies, TextAlign, TextDirection, TextStyle};
use crate::{
    interop::{self, AsStr, FromStrs, SetStr},
    modules::paragraph::TextHeightBehavior,
    prelude::*,
    scalar, FontStyle,
};
use skia_bindings as sb;

pub type StrutStyle = Handle<sb::skia_textlayout_StrutStyle>;
unsafe impl Send for StrutStyle {}
unsafe impl Sync for StrutStyle {}

impl NativeDrop for sb::skia_textlayout_StrutStyle {
    fn drop(&mut self) {
        unsafe { sb::C_StrutStyle_destruct(self) }
    }
}

impl NativeClone for sb::skia_textlayout_StrutStyle {
    fn clone(&self) -> Self {
        construct(|ss| unsafe { sb::C_StrutStyle_CopyConstruct(ss, self) })
    }
}

impl NativePartialEq for sb::skia_textlayout_StrutStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_StrutStyle_equals(self, rhs) }
    }
}

impl Default for StrutStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for StrutStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StrutStyle")
            .field("font_families", &self.font_families())
            .field("font_style", &self.font_style())
            .field("font_size", &self.font_size())
            .field("height", &self.height())
            .field("leading", &self.leading())
            .field("strut_enabled", &self.strut_enabled())
            .field("force_strut_height", &self.force_strut_height())
            .field("height_override", &self.height_override())
            .field("half_leading", &self.half_leading())
            .finish()
    }
}

impl StrutStyle {
    pub fn new() -> Self {
        StrutStyle::construct(|ss| unsafe { sb::C_StrutStyle_Construct(ss) })
    }

    pub fn font_families(&self) -> FontFamilies {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_StrutStyle_getFontFamilies(self.native(), &mut count);
            FontFamilies(safer::from_raw_parts(ptr, count))
        }
    }

    pub fn set_font_families(&mut self, families: &[impl AsRef<str>]) -> &mut Self {
        let families: Vec<interop::String> = FromStrs::from_strs(families);
        let families = families.native();
        unsafe {
            sb::C_StrutStyle_setFontFamilies(self.native_mut(), families.as_ptr(), families.len());
        }
        self
    }

    pub fn font_style(&self) -> FontStyle {
        FontStyle::from_native_c(self.native().fFontStyle)
    }

    pub fn set_font_style(&mut self, font_style: FontStyle) -> &mut Self {
        self.native_mut().fFontStyle = font_style.into_native();
        self
    }

    pub fn font_size(&self) -> scalar {
        self.native().fFontSize
    }

    pub fn set_font_size(&mut self, font_size: scalar) -> &mut Self {
        self.native_mut().fFontSize = font_size;
        self
    }

    pub fn set_height(&mut self, height: scalar) -> &mut Self {
        self.native_mut().fHeight = height;
        self
    }

    pub fn height(&self) -> scalar {
        self.native().fHeight
    }

    pub fn set_leading(&mut self, leading: scalar) -> &mut Self {
        self.native_mut().fLeading = leading;
        self
    }

    pub fn leading(&self) -> scalar {
        self.native().fLeading
    }

    pub fn strut_enabled(&self) -> bool {
        self.native().fEnabled
    }

    pub fn set_strut_enabled(&mut self, enabled: bool) -> &mut Self {
        self.native_mut().fEnabled = enabled;
        self
    }

    pub fn force_strut_height(&self) -> bool {
        self.native().fForceHeight
    }

    pub fn set_force_strut_height(&mut self, force_height: bool) -> &mut Self {
        self.native_mut().fForceHeight = force_height;
        self
    }

    pub fn height_override(&self) -> bool {
        self.native().fHeightOverride
    }

    pub fn set_height_override(&mut self, height_override: bool) -> &mut Self {
        self.native_mut().fHeightOverride = height_override;
        self
    }

    pub fn half_leading(&self) -> bool {
        self.native().fHalfLeading
    }

    pub fn set_half_leading(&mut self, half_leading: bool) -> &mut Self {
        self.native_mut().fHalfLeading = half_leading;
        self
    }
}

// Can't use Handle<> here, std::u16string maintains an interior pointer.
pub type ParagraphStyle = RefHandle<sb::skia_textlayout_ParagraphStyle>;
unsafe impl Send for ParagraphStyle {}
unsafe impl Sync for ParagraphStyle {}

impl NativeDrop for sb::skia_textlayout_ParagraphStyle {
    fn drop(&mut self) {
        unsafe { sb::C_ParagraphStyle_delete(self) }
    }
}

impl Clone for ParagraphStyle {
    fn clone(&self) -> Self {
        Self::from_ptr(unsafe { sb::C_ParagraphStyle_NewCopy(self.native()) }).unwrap()
    }
}

impl NativePartialEq for sb::skia_textlayout_ParagraphStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_ParagraphStyle_Equals(self, rhs) }
    }
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ParagraphStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParagraphStyle")
            .field("strut_style", &self.strut_style())
            .field("text_style", &self.text_style())
            .field("text_direction", &self.text_direction())
            .field("text_align", &self.text_align())
            .field("max_lines", &self.max_lines())
            .field("ellipsis", &self.ellipsis())
            .field("height", &self.height())
            .field("text_height_behavior", &self.text_height_behavior())
            .field("unlimited_lines", &self.unlimited_lines())
            .field("ellipsized", &self.ellipsized())
            .field("effective_align", &self.effective_align())
            .field("hinting_is_on", &self.hinting_is_on())
            .field("draw_options", &self.draw_options())
            .finish()
    }
}

impl ParagraphStyle {
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_ParagraphStyle_New() }).unwrap()
    }

    pub fn strut_style(&self) -> &StrutStyle {
        StrutStyle::from_native_ref(&self.native().fStrutStyle)
    }

    pub fn set_strut_style(&mut self, strut_style: StrutStyle) -> &mut Self {
        self.native_mut().fStrutStyle.replace_with(strut_style);
        self
    }

    pub fn text_style(&self) -> &TextStyle {
        TextStyle::from_native_ref(&self.native().fDefaultTextStyle)
    }

    pub fn set_text_style(&mut self, text_style: &TextStyle) -> &mut Self {
        // TODO: implement the assignment operator in C.
        self.native_mut()
            .fDefaultTextStyle
            .replace_with(text_style.clone());
        self
    }

    pub fn text_direction(&self) -> TextDirection {
        self.native().fTextDirection
    }

    pub fn set_text_direction(&mut self, direction: TextDirection) -> &mut Self {
        self.native_mut().fTextDirection = direction;
        self
    }

    pub fn text_align(&self) -> TextAlign {
        self.native().fTextAlign
    }

    pub fn set_text_align(&mut self, align: TextAlign) -> &mut Self {
        self.native_mut().fTextAlign = align;
        self
    }

    pub fn max_lines(&self) -> Option<usize> {
        match self.native().fLinesLimit {
            std::usize::MAX => None,
            lines => Some(lines),
        }
    }

    pub fn set_max_lines(&mut self, lines: impl Into<Option<usize>>) -> &mut Self {
        self.native_mut().fLinesLimit = lines.into().unwrap_or(usize::max_value());
        self
    }

    // TODO: Support u16 ellipsis, but why? Doesn't SkString support UTF-8?

    pub fn ellipsis(&self) -> &str {
        self.native().fEllipsis.as_str()
    }

    pub fn set_ellipsis(&mut self, ellipsis: impl AsRef<str>) -> &mut Self {
        self.native_mut().fEllipsis.set_str(ellipsis);
        self
    }

    pub fn height(&self) -> scalar {
        self.native().fHeight
    }

    pub fn set_height(&mut self, height: scalar) -> &mut Self {
        self.native_mut().fHeight = height;
        self
    }

    pub fn text_height_behavior(&self) -> TextHeightBehavior {
        self.native().fTextHeightBehavior
    }

    pub fn set_text_height_behavior(&mut self, v: TextHeightBehavior) -> &mut Self {
        self.native_mut().fTextHeightBehavior = v;
        self
    }

    pub fn unlimited_lines(&self) -> bool {
        self.max_lines().is_none()
    }

    pub fn ellipsized(&self) -> bool {
        unsafe { sb::C_ParagraphStyle_ellipsized(self.native()) }
    }

    pub fn effective_align(&self) -> TextAlign {
        unsafe { self.native().effective_align() }
    }

    pub fn hinting_is_on(&self) -> bool {
        self.native().fHintingIsOn
    }

    pub fn turn_hinting_off(&mut self) -> &mut Self {
        self.native_mut().fHintingIsOn = false;
        self
    }

    pub fn draw_options(&self) -> DrawOptions {
        self.native().fDrawingOptions
    }

    pub fn set_draw_options(&mut self, value: DrawOptions) -> &mut Self {
        self.native_mut().fDrawingOptions = value;
        self
    }
}
