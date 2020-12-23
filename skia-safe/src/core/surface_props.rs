use crate::prelude::*;
use skia_bindings as sb;
use skia_bindings::{SkPixelGeometry, SkSurfaceProps};

// TODO: use the enum rewriter and strip underscores?
/// The subpixel layout of the display (for subpixel anti-aliasing)
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum PixelGeometry {
    /// Unknown subpixel layout (this will disable subpixel anti-aliasing entirely). Using
    /// this may lead to text and fine details having a blurry look.
    Unknown = SkPixelGeometry::kUnknown_SkPixelGeometry as _,
    RGBH = SkPixelGeometry::kRGB_H_SkPixelGeometry as _,
    BGRH = SkPixelGeometry::kBGR_H_SkPixelGeometry as _,
    RGBV = SkPixelGeometry::kRGB_V_SkPixelGeometry as _,
    BGRV = SkPixelGeometry::kBGR_V_SkPixelGeometry as _,
}

impl NativeTransmutable<SkPixelGeometry> for PixelGeometry {}

impl PixelGeometry {
    pub fn is_rgb(self) -> bool {
        self == PixelGeometry::RGBH || self == PixelGeometry::RGBV
    }

    pub fn is_bgr(self) -> bool {
        self == PixelGeometry::BGRH || self == PixelGeometry::BGRV
    }

    pub fn is_h(self) -> bool {
        self == PixelGeometry::RGBH || self == PixelGeometry::BGRH
    }

    pub fn is_v(self) -> bool {
        self == PixelGeometry::RGBV || self == PixelGeometry::BGRV
    }
}

impl Default for PixelGeometry {
    fn default() -> Self {
        PixelGeometry::Unknown
    }
}

bitflags! {
    pub struct SurfacePropsFlags: u32 {
        const USE_DEVICE_INDEPENDENT_FONTS =
            sb::SkSurfaceProps_Flags_kUseDeviceIndependentFonts_Flag as u32;
    }
}

impl Default for SurfacePropsFlags {
    fn default() -> Self {
        SurfacePropsFlags::empty()
    }
}

/// The properties of a surface - flags (see `SurfacePropsFlags` and the pixel geometry for
/// subpixel anti-aliasing.
#[derive(Copy)]
#[repr(transparent)]
pub struct SurfaceProps(SkSurfaceProps);

impl NativeTransmutable<SkSurfaceProps> for SurfaceProps {}

impl Clone for SurfaceProps {
    fn clone(&self) -> Self {
        Self::from_native_c(unsafe { SkSurfaceProps::new3(self.native()) })
    }
}

impl PartialEq for SurfaceProps {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_SkSurfaceProps_Equals(self.native(), other.native()) }
    }
}

impl Eq for SurfaceProps {}

impl Default for SurfaceProps {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceProps {
    // TODO: do we need to wrap the construcor(s) with InitType?

    /// Initialize a new `SurfaceProps` value with the default flags and pixel geometry.
    pub fn new() -> Self {
        Self::from_native_c(unsafe { SkSurfaceProps::new() })
    }

    /// Initialize a new `SurfaceProps` value, explicitly setting the flags and pixel geometry of the
    /// surface.
    pub fn with_options(flags: SurfacePropsFlags, pixel_geometry: PixelGeometry) -> SurfaceProps {
        Self::from_native_c(unsafe {
            SkSurfaceProps::new1(flags.bits(), pixel_geometry.into_native())
        })
    }

    /// Get the surface flags. See `SurfacePropsFlags` for more details.
    pub fn flags(self) -> SurfacePropsFlags {
        SurfacePropsFlags::from_bits_truncate(self.native().fFlags)
    }

    /// Get the pixel geomtry of the surface. This is used for subpixel anti-aliasing.
    pub fn pixel_geometry(self) -> PixelGeometry {
        PixelGeometry::from_native_c(self.native().fPixelGeometry)
    }

    /// Check `self.flags()`, returning `true` if this surface is marked to use device-independent fonts.
    pub fn is_use_device_independent_fonts(self) -> bool {
        self.flags()
            .contains(SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS)
    }
}

#[cfg(test)]
mod tests {
    use super::{PixelGeometry, SurfaceProps, SurfacePropsFlags};
    use crate::prelude::NativeTransmutable;

    #[test]
    fn test_pixel_geometry_layout() {
        PixelGeometry::test_layout()
    }

    #[test]
    fn test_surface_props_layout() {
        SurfaceProps::test_layout()
    }

    #[test]
    fn create() {
        let props = SurfaceProps::with_options(
            SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS,
            PixelGeometry::RGBH,
        );
        assert_eq!(
            SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS,
            props.flags()
        );
        assert_eq!(PixelGeometry::RGBH, props.pixel_geometry());
        assert_eq!(true, props.is_use_device_independent_fonts());
    }
}
