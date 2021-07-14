use super::{PositionWithAffinity, RectHeightStyle, RectWidthStyle, TextBox};
use crate::{interop::VecSink, prelude::*, scalar, textlayout::LineMetrics, Canvas, Point};
use skia_bindings as sb;
use std::{fmt, ops::Range};

pub type Paragraph = RefHandle<sb::skia_textlayout_Paragraph>;
unsafe impl Send for Paragraph {}
unsafe impl Sync for Paragraph {}

impl NativeDrop for sb::skia_textlayout_Paragraph {
    fn drop(&mut self) {
        unsafe { sb::C_Paragraph_delete(self) }
    }
}

impl fmt::Debug for Paragraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paragraph")
            .field("max_width", &self.max_width())
            .field("height", &self.height())
            .field("min_intrinsic_width", &self.min_intrinsic_width())
            .field("max_intrinsic_width", &self.max_intrinsic_width())
            .field("alphabetic_baseline", &self.alphabetic_baseline())
            .field("ideographic_baseline", &self.ideographic_baseline())
            .field("longest_line", &self.longest_line())
            .field("did_exceed_max_lines", &self.did_exceed_max_lines())
            .field("line_number", &self.line_number())
            .finish()
    }
}

impl Paragraph {
    pub fn max_width(&self) -> scalar {
        self.native().fWidth
    }

    pub fn height(&self) -> scalar {
        self.native().fHeight
    }

    pub fn min_intrinsic_width(&self) -> scalar {
        self.native().fMinIntrinsicWidth
    }

    pub fn max_intrinsic_width(&self) -> scalar {
        self.native().fMaxIntrinsicWidth
    }

    pub fn alphabetic_baseline(&self) -> scalar {
        self.native().fAlphabeticBaseline
    }

    pub fn ideographic_baseline(&self) -> scalar {
        self.native().fIdeographicBaseline
    }

    pub fn longest_line(&self) -> scalar {
        self.native().fLongestLine
    }

    pub fn did_exceed_max_lines(&self) -> bool {
        self.native().fExceededMaxLines
    }

    pub fn layout(&mut self, width: scalar) {
        unsafe { sb::C_Paragraph_layout(self.native_mut(), width) }
    }

    pub fn paint(&self, canvas: &mut Canvas, p: impl Into<Point>) {
        let p = p.into();
        unsafe { sb::C_Paragraph_paint(self.native_mut_force(), canvas.native_mut(), p.x, p.y) }
    }

    pub fn get_rects_for_range(
        &self,
        range: Range<usize>,
        rect_height_style: RectHeightStyle,
        rect_width_style: RectWidthStyle,
    ) -> Vec<TextBox> {
        let mut result: Vec<TextBox> = Vec::new();

        let mut set_tb = |tbs: &[sb::skia_textlayout_TextBox]| {
            result = tbs
                .iter()
                .map(|tb| TextBox::from_native_ref(tb))
                .cloned()
                .collect();
        };

        unsafe {
            sb::C_Paragraph_getRectsForRange(
                self.native_mut_force(),
                range.start.try_into().unwrap(),
                range.end.try_into().unwrap(),
                rect_height_style,
                rect_width_style,
                VecSink::new(&mut set_tb).native_mut(),
            );
        }
        result
    }

    pub fn get_rects_for_placeholders(&self) -> Vec<TextBox> {
        let mut result: Vec<TextBox> = Vec::new();

        let mut set_tb = |tbs: &[sb::skia_textlayout_TextBox]| {
            result = tbs
                .iter()
                .map(|tb| TextBox::from_native_ref(tb))
                .cloned()
                .collect();
        };

        unsafe {
            sb::C_Paragraph_getRectsForPlaceholders(
                self.native_mut_force(),
                VecSink::new(&mut set_tb).native_mut(),
            )
        }
        result
    }

    pub fn get_glyph_position_at_coordinate(&self, p: impl Into<Point>) -> PositionWithAffinity {
        let p = p.into();
        let mut r = Default::default();
        unsafe {
            sb::C_Paragraph_getGlyphPositionAtCoordinate(self.native_mut_force(), p.x, p.y, &mut r)
        }
        r
    }

    pub fn get_word_boundary(&self, offset: u32) -> Range<usize> {
        let mut range: [usize; 2] = Default::default();
        unsafe {
            sb::C_Paragraph_getWordBoundary(self.native_mut_force(), offset, range.as_mut_ptr())
        }
        range[0]..range[1]
    }

    pub fn get_line_metrics(&self) -> Vec<LineMetrics> {
        let mut result: Vec<LineMetrics> = Vec::new();
        let mut set_lm = |lms: &[sb::skia_textlayout_LineMetrics]| {
            result = lms
                .iter()
                .map(|lm| LineMetrics::from_native_ref(lm).clone())
                .collect();
        };

        unsafe {
            sb::C_Paragraph_getLineMetrics(
                self.native_mut_force(),
                VecSink::new(&mut set_lm).native_mut(),
            )
        }

        result
    }

    pub fn line_number(&self) -> usize {
        unsafe { sb::C_Paragraph_lineNumber(self.native_mut_force()) }
    }

    pub fn mark_dirty(&mut self) {
        unsafe { sb::C_Paragraph_markDirty(self.native_mut()) }
    }

    pub fn unresolved_glyphs(&mut self) -> Option<usize> {
        unsafe { sb::C_Paragraph_unresolvedGlyphs(self.native_mut()) }
            .try_into()
            .ok()
    }

    // TODO: wrap visit()
}

#[deprecated(since = "0.41.0", note = "Use Vec<TextBox>")]
pub type TextBoxes = Vec<TextBox>;

#[deprecated(since = "0.41.0", note = "Use Vec<LineMetrics>")]
pub type LineMetricsVector<'a> = Vec<LineMetrics<'a>>;

#[cfg(test)]
mod tests {
    use crate::{
        icu,
        textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle},
        FontMgr,
    };

    #[test]
    #[serial_test::serial]
    fn test_line_metrics() {
        icu::init();

        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(FontMgr::new(), None);
        let paragraph_style = ParagraphStyle::new();
        let mut paragraph_builder = ParagraphBuilder::new(&paragraph_style, font_collection);
        let ts = TextStyle::new();
        paragraph_builder.push_style(&ts);
        paragraph_builder.add_text(LOREM_IPSUM);
        let mut paragraph = paragraph_builder.build();
        paragraph.layout(256.0);

        let line_metrics = paragraph.get_line_metrics();
        for (line, lm) in line_metrics.iter().enumerate() {
            println!("line {}: width: {}", line + 1, lm.width)
        }

        static LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur at leo at nulla tincidunt placerat. Proin eget purus augue. Quisque et est ullamcorper, pellentesque felis nec, pulvinar massa. Aliquam imperdiet, nulla ut dictum euismod, purus dui pulvinar risus, eu suscipit elit neque ac est. Nullam eleifend justo quis placerat ultricies. Vestibulum ut elementum velit. Praesent et dolor sit amet purus bibendum mattis. Aliquam erat volutpat.";
    }
}
