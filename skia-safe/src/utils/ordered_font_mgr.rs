use crate::{prelude::*, FontMgr};
use skia_bindings::{self as sb, SkOrderedFontMgr, SkRefCntBase};
use std::{
    fmt,
    mem::transmute,
    ops::{Deref, DerefMut},
};

pub type OrderedFontMgr = RCHandle<SkOrderedFontMgr>;

impl NativeRefCountedBase for SkOrderedFontMgr {
    type Base = SkRefCntBase;
}

impl Deref for OrderedFontMgr {
    type Target = FontMgr;
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}

impl DerefMut for OrderedFontMgr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_ref_mut(self) }
    }
}

impl Default for OrderedFontMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl From<OrderedFontMgr> for FontMgr {
    fn from(font_mgr: OrderedFontMgr) -> Self {
        unsafe { transmute(font_mgr) }
    }
}

impl fmt::Debug for OrderedFontMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderedFontMgr")
            .field("base", self as &FontMgr)
            .finish()
    }
}

impl OrderedFontMgr {
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_SkOrderedFontMgr_new() }).unwrap()
    }

    pub fn append(&mut self, font_mgr: impl Into<FontMgr>) {
        let font_mgr = font_mgr.into();
        unsafe { sb::C_SkOrderedFontMgr_append(self.native_mut(), font_mgr.into_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedFontMgr;

    #[test]
    fn can_use_font_mgr_functions() {
        let ordered = OrderedFontMgr::default();
        let _families = ordered.count_families();
    }

    #[test]
    fn can_pass_ordered_font_mgr_where_a_font_mgr_is_expected() {
        let mut ordered = OrderedFontMgr::default();
        let another = OrderedFontMgr::default();
        ordered.append(another);
    }
}
