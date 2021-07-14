use crate::{paragraph::TextStyle, prelude::*, FontMetrics};
use skia_bindings::{self as sb, skia_textlayout_LineMetrics, skia_textlayout_StyleMetrics};
use std::{marker, mem, ops::Range, ptr};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct StyleMetrics<'a> {
    pub text_style: &'a TextStyle,
    pub font_metrics: FontMetrics,
}

impl NativeTransmutable<skia_textlayout_StyleMetrics> for StyleMetrics<'_> {}

#[test]
fn test_style_metrics_layout() {
    StyleMetrics::test_layout();
}

impl<'a> StyleMetrics<'a> {
    pub fn new(style: &'a TextStyle, metrics: impl Into<Option<FontMetrics>>) -> Self {
        Self {
            text_style: style,
            font_metrics: metrics.into().unwrap_or_default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct LineMetrics<'a> {
    pub start_index: usize,
    pub end_index: usize,
    pub end_excluding_whitespaces: usize,
    pub end_including_newline: usize,
    pub hard_break: bool,
    pub ascent: f64,
    pub descent: f64,
    pub unscaled_ascent: f64,
    pub height: f64,
    pub width: f64,
    pub left: f64,
    pub baseline: f64,
    pub line_number: usize,
    line_metrics:
        [u8; mem::size_of::<skia_textlayout_LineMetrics>() - mem::size_of::<LMInternal>()],
    pd: marker::PhantomData<&'a StyleMetrics<'a>>,
}

impl NativeTransmutable<skia_textlayout_LineMetrics> for LineMetrics<'_> {}

// Internal Line Metrics mirror to compute what the map takes up space.
// If the size of the structure does not match, the NativeTransmutable test below will fail.
#[repr(C)]
struct LMInternal {
    start_end: [usize; 4],
    hard_break: bool,
    seven_metrics: [f64; 7],
    line_number: usize,
}

#[test]
fn test_line_metrics_layout() {
    LineMetrics::test_layout();
}

impl<'a> LineMetrics<'a> {
    // TODO: may support constructors (but what about the lifetime bounds?).

    /// Returns the number of style metrics in the given index range.
    pub fn get_style_metrics_count(&self, range: Range<usize>) -> usize {
        unsafe { sb::C_LineMetrics_fLineMetrics_count(self.native(), range.start, range.end) }
    }

    /// Returns indices and references to style metrics in the given range.
    pub fn get_style_metrics(&self, range: Range<usize>) -> Vec<StyleMetricsRecord<'a>> {
        let count = self.get_style_metrics_count(range.clone());
        let mut v: Vec<(usize, *mut StyleMetrics<'a>)> = vec![(0, ptr::null_mut()); count];
        unsafe {
            sb::C_LineMetrics_fLineMetrics_getRange(
                self.native(),
                range.start,
                range.end,
                v.as_mut_ptr() as *mut sb::StyleMetricsRecord,
            );
            // TODO: can the second allocation of that vec be avoided? Transmuting the vec is
            //       UB beginning with Rust 1.40.
            v.into_iter()
                .map(|v| mem::transmute::<(usize, *mut StyleMetrics<'a>), StyleMetricsRecord>(v))
                .collect()
        }
    }
}

type StyleMetricsRecord<'a> = (usize, &'a StyleMetrics<'a>);

#[test]
fn test_style_metrics_record_layout() {
    assert_eq!(
        mem::size_of::<(usize, *mut StyleMetrics)>(),
        mem::size_of::<sb::StyleMetricsRecord>()
    )
}
