#![cfg(feature = "textlayout")]
use skia_safe::shaper::run_handler::{Buffer, RunInfo};
use skia_safe::shaper::RunHandler;
use skia_safe::{Font, GlyphId, Point, Shaper};

#[derive(Default, Debug)]
pub struct DebugRunHandler {
    glyphs: Vec<GlyphId>,
    points: Vec<Point>,
}

impl RunHandler for DebugRunHandler {
    fn begin_line(&mut self) {
        println!("begin_line");
    }

    fn run_info(&mut self, info: &RunInfo) {
        println!("run_info: {:?} {:?}", info.advance, info.utf8_range);
    }

    fn commit_run_info(&mut self) {
        println!("commit_run_info");
    }

    fn run_buffer<'a>(&'a mut self, info: &RunInfo) -> Buffer {
        println!("run_buffer {}", info.glyph_count);
        let count = info.glyph_count;
        self.glyphs.resize(count, 0);
        self.points.resize(count, Point::default());
        Buffer::new(&mut self.glyphs, &mut self.points, None)
    }

    fn commit_run_buffer(&mut self, _info: &RunInfo) {
        println!("commit_run_buffer");
        println!("state: {:?}", self);
    }

    fn commit_line(&mut self) {
        println!("commit_line");
    }
}

#[test]
#[serial_test::serial]
fn test_rtl_text_shaping() {
    let shaper = Shaper::new(None);
    shaper.shape(
        "العربية",
        &Font::default(),
        false,
        10000.0,
        &mut DebugRunHandler::default(),
    );
}
