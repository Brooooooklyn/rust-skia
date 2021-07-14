//! This file contains Default trait implementations for types that are
//! re-exported from skia-safe.

use crate::{
    SkBlendMode, SkBlurStyle, SkCanvas_Lattice_RectType, SkClipOp, SkPaint_Cap, SkPaint_Join,
    SkParsePath_PathEncoding, SkPathDirection, SkTileMode, SkYUVColorSpace,
};

impl Default for SkBlendMode {
    fn default() -> Self {
        SkBlendMode::SrcOver
    }
}

impl Default for SkPaint_Cap {
    fn default() -> Self {
        SkPaint_Cap::Default
    }
}

impl Default for SkPaint_Join {
    fn default() -> Self {
        SkPaint_Join::Default
    }
}

impl Default for SkBlurStyle {
    fn default() -> Self {
        SkBlurStyle::Normal
    }
}

impl Default for SkCanvas_Lattice_RectType {
    fn default() -> Self {
        SkCanvas_Lattice_RectType::Default
    }
}

// This is the default for the canvas's clip functions.
impl Default for SkClipOp {
    fn default() -> Self {
        SkClipOp::Intersect
    }
}

impl Default for SkYUVColorSpace {
    fn default() -> Self {
        SkYUVColorSpace::Identity
    }
}

impl Default for SkPathDirection {
    fn default() -> Self {
        SkPathDirection::CW
    }
}

impl Default for SkTileMode {
    fn default() -> Self {
        SkTileMode::Clamp
    }
}

impl Default for SkParsePath_PathEncoding {
    fn default() -> Self {
        SkParsePath_PathEncoding::Absolute
    }
}

#[cfg(feature = "textlayout")]
pub mod textlayout {
    impl Default for crate::skia_textlayout_Affinity {
        fn default() -> Self {
            Self::Upstream
        }
    }

    impl Default for crate::skia_textlayout_RectHeightStyle {
        fn default() -> Self {
            Self::Tight
        }
    }

    impl Default for crate::skia_textlayout_RectWidthStyle {
        fn default() -> Self {
            Self::Tight
        }
    }

    impl Default for crate::skia_textlayout_TextAlign {
        fn default() -> Self {
            Self::Left
        }
    }

    impl Default for crate::skia_textlayout_PositionWithAffinity {
        fn default() -> Self {
            Self {
                position: 0,
                affinity: Default::default(),
            }
        }
    }

    impl Default for crate::skia_textlayout_TextBaseline {
        fn default() -> Self {
            Self::Alphabetic
        }
    }

    impl Default for crate::skia_textlayout_TextDecorationStyle {
        fn default() -> Self {
            Self::Solid
        }
    }

    impl Default for crate::skia_textlayout_TextDecorationMode {
        fn default() -> Self {
            Self::Gaps
        }
    }

    impl Default for crate::skia_textlayout_StyleType {
        fn default() -> Self {
            Self::AllAttributes
        }
    }

    impl Default for crate::skia_textlayout_PlaceholderAlignment {
        fn default() -> Self {
            Self::Baseline
        }
    }
}
