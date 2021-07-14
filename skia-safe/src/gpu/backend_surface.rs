#[cfg(feature = "d3d")]
use super::d3d;
#[cfg(feature = "gl")]
use super::gl;
#[cfg(feature = "metal")]
use super::mtl;
#[cfg(feature = "vulkan")]
use super::vk;
use super::{BackendAPI, BackendSurfaceMutableState, Mipmapped};
use crate::{prelude::*, ISize};
use skia_bindings::{
    self as sb, GrBackendFormat, GrBackendRenderTarget, GrBackendTexture, GrMipmapped,
};
use std::fmt;

pub type BackendFormat = Handle<GrBackendFormat>;
unsafe impl Send for BackendFormat {}
unsafe impl Sync for BackendFormat {}

impl NativeDrop for GrBackendFormat {
    fn drop(&mut self) {
        unsafe { sb::C_GrBackendFormat_destruct(self) }
    }
}

impl NativeClone for GrBackendFormat {
    fn clone(&self) -> Self {
        unsafe { GrBackendFormat::new(self) }
    }
}

impl fmt::Debug for BackendFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("BackendFormat");
        d.field("backend", &self.backend());
        d.field("channel_mask", &self.channel_mask());
        #[cfg(feature = "gl")]
        d.field("gl_format", &self.as_gl_format());
        #[cfg(feature = "vulkan")]
        d.field("vk_format", &self.as_vk_format());
        #[cfg(feature = "metal")]
        d.field("mtl_format", &self.as_mtl_format());
        #[cfg(feature = "d3d")]
        d.field("dxgi_format", &self.as_dxgi_format());
        d.finish()
    }
}

impl BackendFormat {
    #[deprecated(
        note = "The creation of invalid BackendFormats isn't supported anymore",
        since = "0.37.0"
    )]
    pub fn new() -> Self {
        Self::new_invalid()
    }

    pub(crate) fn new_invalid() -> Self {
        Self::construct(|bf| unsafe { sb::C_GrBackendFormat_Construct(bf) })
    }

    #[cfg(feature = "gl")]
    pub fn new_gl(format: gl::Enum, target: gl::Enum) -> Self {
        Self::construct(|bf| unsafe { sb::C_GrBackendFormat_ConstructGL(bf, format, target) })
            .assert_valid()
    }

    #[cfg(feature = "vulkan")]
    pub fn new_vulkan(format: vk::Format) -> Self {
        Self::construct(|bf| unsafe { sb::C_GrBackendFormat_ConstructVk(bf, format) })
            .assert_valid()
    }

    #[cfg(feature = "vulkan")]
    pub fn new_vulkan_ycbcr(conversion_info: &vk::YcbcrConversionInfo) -> Self {
        Self::construct(|bf| unsafe {
            sb::C_GrBackendFormat_ConstructVk2(bf, conversion_info.native())
        })
        .assert_valid()
    }

    #[cfg(feature = "metal")]
    #[allow(clippy::missing_safety_doc)]
    pub fn new_metal(format: mtl::PixelFormat) -> Self {
        Self::construct(|bf| unsafe { sb::C_GrBackendFormat_ConstructMtl(bf, format) })
            .assert_valid()
    }

    #[cfg(feature = "d3d")]
    pub fn new_dxgi(format: d3d::DXGI_FORMAT) -> Self {
        Self::construct(|bf| unsafe {
            sb::C_GrBackendFormat_ConstructDxgi(bf, format.into_native())
        })
        .assert_valid()
    }

    pub fn backend(&self) -> BackendAPI {
        self.native().fBackend
    }

    pub fn channel_mask(&self) -> u32 {
        unsafe { self.native().channelMask() }
    }

    #[cfg(feature = "gl")]
    pub fn as_gl_format(&self) -> gl::Format {
        unsafe {
            #[allow(clippy::map_clone)]
            self.native().asGLFormat()
        }
    }

    #[cfg(feature = "vulkan")]
    pub fn as_vk_format(&self) -> Option<vk::Format> {
        let mut r = vk::Format::UNDEFINED;
        unsafe { self.native().asVkFormat(&mut r) }.if_true_some(r)
    }

    #[cfg(feature = "metal")]
    pub fn as_mtl_format(&self) -> Option<mtl::PixelFormat> {
        let pixel_format = unsafe { self.native().asMtlFormat() };
        // Mtl's PixelFormat == 0 is invalid.
        (pixel_format != 0).if_true_some(pixel_format)
    }

    #[cfg(feature = "d3d")]
    pub fn as_dxgi_format(&self) -> Option<d3d::DXGI_FORMAT> {
        let mut f = sb::DXGI_FORMAT::DXGI_FORMAT_UNKNOWN;
        unsafe { self.native().asDxgiFormat(&mut f) }
            .if_true_some(d3d::DXGI_FORMAT::from_native_c(f))
    }

    pub fn to_texture_2d(&self) -> Self {
        let mut new = Self::new_invalid();
        unsafe { sb::C_GrBackendFormat_makeTexture2D(self.native(), new.native_mut()) };
        assert!(Self::native_is_valid(new.native()));
        new
    }

    #[deprecated(
        note = "Invalid BackendFormats are not supported anymore",
        since = "0.37.0"
    )]
    pub fn is_valid(&self) -> bool {
        self.native().fValid
    }

    pub(crate) fn native_is_valid(format: &GrBackendFormat) -> bool {
        format.fValid
    }

    pub(crate) fn assert_valid(self) -> Self {
        assert!(Self::native_is_valid(self.native()));
        self
    }
}

pub type BackendTexture = Handle<GrBackendTexture>;
unsafe impl Send for BackendTexture {}
unsafe impl Sync for BackendTexture {}

impl NativeDrop for GrBackendTexture {
    fn drop(&mut self) {
        unsafe { sb::C_GrBackendTexture_destruct(self) }
    }
}

impl NativeClone for GrBackendTexture {
    fn clone(&self) -> Self {
        construct(|texture| unsafe { sb::C_GrBackendTexture_CopyConstruct(texture, self) })
    }
}

impl fmt::Debug for BackendTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("BackendTexture");
        d.field("dimensions", &self.dimensions());
        d.field("mipmapped", &self.mipmapped());
        d.field("backend", &self.backend());
        #[cfg(feature = "gl")]
        d.field("gl_texture_info", &self.gl_texture_info());
        #[cfg(feature = "vulkan")]
        d.field("vulkan_image_info", &self.vulkan_image_info());
        #[cfg(feature = "metal")]
        d.field("metal_texture_info", &self.metal_texture_info());
        #[cfg(feature = "d3d")]
        d.field(
            "d3d_texture_resource_info",
            &self.d3d_texture_resource_info(),
        );
        d.field("backend_format", &self.backend_format());
        d.field("is_protected", &self.is_protected());
        d.finish()
    }
}

impl BackendTexture {
    pub(crate) fn new_invalid() -> Self {
        Self::construct(|t| unsafe { sb::C_GrBackendTexture_Construct(t) })
    }

    #[cfg(feature = "gl")]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_gl(
        (width, height): (i32, i32),
        mipmapped: super::Mipmapped,
        gl_info: gl::TextureInfo,
    ) -> Self {
        Self::from_native_if_valid(construct(|texture| {
            sb::C_GrBackendTexture_ConstructGL(texture, width, height, mipmapped, gl_info.native())
        }))
        .unwrap()
    }

    #[cfg(feature = "vulkan")]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_vulkan((width, height): (i32, i32), vk_info: &vk::ImageInfo) -> Self {
        Self::from_native_if_valid(construct(|texture| {
            sb::C_GrBackendTexture_ConstructVk(texture, width, height, vk_info.native())
        }))
        .unwrap()
    }

    #[cfg(feature = "metal")]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_metal(
        (width, height): (i32, i32),
        mipmapped: super::Mipmapped,
        mtl_info: &mtl::TextureInfo,
    ) -> Self {
        Self::from_native_if_valid(construct(|texture| {
            sb::C_GrBackendTexture_ConstructMtl(
                texture,
                width,
                height,
                mipmapped,
                mtl_info.native(),
            )
        }))
        .unwrap()
    }

    #[cfg(feature = "d3d")]
    pub fn new_d3d((width, height): (i32, i32), d3d_info: &d3d::TextureResourceInfo) -> Self {
        unsafe {
            Self::from_native_if_valid(construct(|texture| {
                sb::C_GrBackendTexture_ConstructD3D(texture, width, height, d3d_info.native())
            }))
        }
        .unwrap()
    }

    pub(crate) unsafe fn from_native_if_valid(
        backend_texture: GrBackendTexture,
    ) -> Option<BackendTexture> {
        Self::native_is_valid(&backend_texture)
            .if_true_then_some(|| BackendTexture::from_native_c(backend_texture))
    }

    pub fn dimensions(&self) -> ISize {
        ISize::new(self.width(), self.height())
    }

    pub fn width(&self) -> i32 {
        self.native().fWidth
    }

    pub fn height(&self) -> i32 {
        self.native().fHeight
    }

    pub fn mipmapped(&self) -> Mipmapped {
        self.native().fMipmapped
    }

    #[deprecated(since = "0.35.0", note = "Use has_mipmaps()")]
    pub fn has_mip_maps(&self) -> bool {
        self.has_mipmaps()
    }

    pub fn has_mipmaps(&self) -> bool {
        self.native().fMipmapped == GrMipmapped::Yes
    }

    pub fn backend(&self) -> BackendAPI {
        self.native().fBackend
    }

    #[cfg(feature = "gl")]
    pub fn gl_texture_info(&self) -> Option<gl::TextureInfo> {
        unsafe {
            let mut texture_info = gl::TextureInfo::default();
            self.native()
                .getGLTextureInfo(texture_info.native_mut())
                .if_true_some(texture_info)
        }
    }

    #[cfg(feature = "gl")]
    pub fn gl_texture_parameters_modified(&mut self) {
        unsafe { self.native_mut().glTextureParametersModified() }
    }

    #[cfg(feature = "vulkan")]
    pub fn vulkan_image_info(&self) -> Option<vk::ImageInfo> {
        unsafe {
            // constructor not available.
            let mut image_info = vk::ImageInfo::default();
            self.native()
                .getVkImageInfo(image_info.native_mut())
                .if_true_some(image_info)
        }
    }

    #[cfg(feature = "vulkan")]
    pub fn set_vulkan_image_layout(&mut self, layout: vk::ImageLayout) -> &mut Self {
        unsafe { self.native_mut().setVkImageLayout(layout) }
        self
    }

    #[cfg(feature = "metal")]
    pub fn metal_texture_info(&self) -> Option<mtl::TextureInfo> {
        unsafe {
            let mut texture_info = mtl::TextureInfo::default();
            self.native()
                .getMtlTextureInfo(texture_info.native_mut())
                .if_true_some(texture_info)
        }
    }

    #[cfg(feature = "d3d")]
    pub fn d3d_texture_resource_info(&self) -> Option<d3d::TextureResourceInfo> {
        unsafe {
            let mut info = sb::GrD3DTextureResourceInfo::default();
            self.native()
                .getD3DTextureResourceInfo(&mut info)
                .if_true_then_some(|| {
                    assert!(!info.fResource.fObject.is_null());
                    d3d::TextureResourceInfo::from_native_c(info)
                })
        }
    }

    #[cfg(feature = "d3d")]
    pub fn set_d3d_resource_state(&mut self, resource_state: d3d::ResourceStateEnum) -> &mut Self {
        unsafe { self.native_mut().setD3DResourceState(resource_state) }
        self
    }

    pub fn backend_format(&self) -> BackendFormat {
        let mut format = BackendFormat::new_invalid();
        unsafe { sb::C_GrBackendTexture_getBackendFormat(self.native(), format.native_mut()) };
        assert!(BackendFormat::native_is_valid(format.native()));
        format
    }

    pub fn is_protected(&self) -> bool {
        unsafe { self.native().isProtected() }
    }

    #[deprecated(
        note = "Invalid BackendTextures aren't supported anymore",
        since = "0.37.0"
    )]
    pub fn is_valid(&self) -> bool {
        self.native().fIsValid
    }

    pub(crate) fn native_is_valid(texture: &GrBackendTexture) -> bool {
        texture.fIsValid
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_same_texture(&mut self, texture: &BackendTexture) -> bool {
        unsafe { self.native_mut().isSameTexture(texture.native()) }
    }
}

pub type BackendRenderTarget = Handle<GrBackendRenderTarget>;
unsafe impl Send for BackendRenderTarget {}
unsafe impl Sync for BackendRenderTarget {}

impl NativeDrop for GrBackendRenderTarget {
    fn drop(&mut self) {
        unsafe { sb::C_GrBackendRenderTarget_destruct(self) }
    }
}

impl NativeClone for GrBackendRenderTarget {
    fn clone(&self) -> Self {
        construct(|render_target| unsafe {
            sb::C_GrBackendRenderTarget_CopyConstruct(render_target, self)
        })
    }
}

impl fmt::Debug for BackendRenderTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("BackendRenderTarget");
        d.field("dimensions", &self.dimensions());
        d.field("sample_count", &self.sample_count());
        d.field("stencil_bits", &self.stencil_bits());
        d.field("backend", &self.backend());
        d.field("is_framebuffer_only", &self.is_framebuffer_only());
        #[cfg(feature = "gl")]
        d.field("gl_framebuffer_info", &self.gl_framebuffer_info());
        #[cfg(feature = "vulkan")]
        d.field("vulkan_image_info", &self.vulkan_image_info());
        #[cfg(feature = "metal")]
        d.field("metal_texture_info", &self.metal_texture_info());
        #[cfg(feature = "d3d")]
        d.field(
            "d3d_texture_resource_info",
            &self.d3d_texture_resource_info(),
        );
        d.field("backend_format", &self.backend_format());
        d.field("is_protected", &self.is_protected());
        d.finish()
    }
}

impl BackendRenderTarget {
    #[cfg(feature = "gl")]
    pub fn new_gl(
        (width, height): (i32, i32),
        sample_count: impl Into<Option<usize>>,
        stencil_bits: usize,
        info: gl::FramebufferInfo,
    ) -> Self {
        Self::construct(|target| unsafe {
            sb::C_GrBackendRenderTarget_ConstructGL(
                target,
                width,
                height,
                sample_count.into().unwrap_or(0).try_into().unwrap(),
                stencil_bits.try_into().unwrap(),
                info.native(),
            )
        })
    }

    #[cfg(feature = "vulkan")]
    pub fn new_vulkan(
        (width, height): (i32, i32),
        sample_count: impl Into<Option<usize>>,
        info: &vk::ImageInfo,
    ) -> Self {
        Self::construct(|target| unsafe {
            sb::C_GrBackendRenderTarget_ConstructVk(
                target,
                width,
                height,
                sample_count.into().unwrap_or(0).try_into().unwrap(),
                info.native(),
            )
        })
    }

    #[cfg(feature = "metal")]
    pub fn new_metal(
        (width, height): (i32, i32),
        sample_cnt: i32,
        mtl_info: &mtl::TextureInfo,
    ) -> Self {
        Self::construct(|target| unsafe {
            sb::C_GrBackendRenderTarget_ConstructMtl(
                target,
                width,
                height,
                sample_cnt,
                mtl_info.native(),
            )
        })
    }

    #[cfg(feature = "d3d")]
    pub fn new_d3d((width, height): (i32, i32), d3d_info: &d3d::TextureResourceInfo) -> Self {
        Self::construct(|brt| unsafe {
            sb::C_GrBackendRenderTarget_ConstructD3D(brt, width, height, d3d_info.native())
        })
    }

    pub(crate) fn from_native_c_if_valid(
        native: GrBackendRenderTarget,
    ) -> Option<BackendRenderTarget> {
        let backend_render_target = BackendRenderTarget::from_native_c(native);
        Self::native_is_valid(backend_render_target.native()).if_true_some(backend_render_target)
    }

    pub fn dimensions(&self) -> ISize {
        ISize::new(self.width(), self.height())
    }

    pub fn width(&self) -> i32 {
        self.native().fWidth
    }

    pub fn height(&self) -> i32 {
        self.native().fHeight
    }

    pub fn sample_count(&self) -> usize {
        self.native().fSampleCnt.try_into().unwrap()
    }

    pub fn stencil_bits(&self) -> usize {
        self.native().fStencilBits.try_into().unwrap()
    }

    pub fn backend(&self) -> BackendAPI {
        self.native().fBackend
    }

    pub fn is_framebuffer_only(&self) -> bool {
        self.native().fFramebufferOnly
    }

    #[cfg(feature = "gl")]
    pub fn gl_framebuffer_info(&self) -> Option<gl::FramebufferInfo> {
        let mut info = gl::FramebufferInfo::default();
        unsafe { self.native().getGLFramebufferInfo(info.native_mut()) }.if_true_some(info)
    }

    #[cfg(feature = "vulkan")]
    pub fn vulkan_image_info(&self) -> Option<vk::ImageInfo> {
        let mut info = vk::ImageInfo::default();
        unsafe { self.native().getVkImageInfo(info.native_mut()) }.if_true_some(info)
    }

    #[cfg(feature = "vulkan")]
    pub fn set_vulkan_image_layout(&mut self, layout: vk::ImageLayout) -> &mut Self {
        unsafe { self.native_mut().setVkImageLayout(layout) }
        self
    }

    #[cfg(feature = "metal")]
    pub fn metal_texture_info(&self) -> Option<mtl::TextureInfo> {
        let mut info = mtl::TextureInfo::default();
        unsafe { self.native().getMtlTextureInfo(info.native_mut()) }.if_true_some(info)
    }

    #[cfg(feature = "d3d")]
    pub fn d3d_texture_resource_info(&self) -> Option<d3d::TextureResourceInfo> {
        let mut info = sb::GrD3DTextureResourceInfo::default();
        unsafe { self.native().getD3DTextureResourceInfo(&mut info) }.if_true_then_some(|| {
            assert!(!info.fResource.fObject.is_null());
            d3d::TextureResourceInfo::from_native_c(info)
        })
    }

    #[cfg(feature = "d3d")]
    pub fn set_d3d_resource_state(&mut self, resource_state: d3d::ResourceStateEnum) -> &mut Self {
        unsafe { self.native_mut().setD3DResourceState(resource_state) }
        self
    }

    pub fn backend_format(&self) -> BackendFormat {
        BackendFormat::construct(|format| unsafe {
            sb::C_GrBackendRenderTarget_getBackendFormat(self.native(), format)
        })
    }

    pub fn set_mutable_stat(&mut self, state: &BackendSurfaceMutableState) {
        unsafe { self.native_mut().setMutableState(state.native()) }
    }

    pub fn is_protected(&self) -> bool {
        unsafe { self.native().isProtected() }
    }

    #[deprecated(since = "0.37.0", note = "BackendRenderTargets must be valid.")]
    pub fn is_valid(&self) -> bool {
        self.native().fIsValid
    }

    pub(crate) fn native_is_valid(rt: &GrBackendRenderTarget) -> bool {
        rt.fIsValid
    }
}
