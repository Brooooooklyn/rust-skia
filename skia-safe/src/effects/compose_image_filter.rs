use crate::ImageFilter;
use skia_bindings::C_SkComposeImageFilter_Make;

pub enum ComposeImageFilter {}

impl ComposeImageFilter {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(outer: &ImageFilter, inner: &ImageFilter) -> Option<ImageFilter> {
        ImageFilter::from_ptr(unsafe {
            C_SkComposeImageFilter_Make(outer.shared_native(), inner.shared_native())
        })
    }
}
