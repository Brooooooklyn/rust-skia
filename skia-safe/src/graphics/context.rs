use crate::prelude::*;
use crate::graphics::gl;
use skia_bindings::{GrContext, SkRefCntBase, C_GrContext_MakeGL, GrContext_abandonContext, GrContext_releaseResourcesAndAbandonContext, GrContext_freeGpuResources};

#[cfg(feature = "vulkan")]
use skia_bindings::C_GrContext_MakeVulkan;
#[cfg(feature = "vulkan")]
use super::vulkan;
use crate::skia::ColorType;

pub type Context = RCHandle<GrContext>;

impl NativeRefCountedBase for GrContext {
    type Base = SkRefCntBase;
    fn ref_counted_base(&self) -> &Self::Base {
        &self._base._base
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ResourceCacheLimits {
    pub max_resources: usize,
    pub max_resource_bytes: usize
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ResourceCacheUsage {
    pub resource_count: usize,
    pub resource_bytes: usize
}

impl RCHandle<GrContext> {

    // TODO: support variant with GrContextOptions
    pub fn new_gl(interface: Option<&gl::Interface>) -> Option<Context> {
        Context::from_ptr(unsafe {
            C_GrContext_MakeGL(interface.native_ptr_or_null())
        })
    }

    // TODO: support variant with GrContextOptions
    #[cfg(feature = "vulkan")]
    pub fn new_vulkan(backend_context: &vulkan::BackendContext) -> Option<Context> {
       Context::from_ptr(unsafe { C_GrContext_MakeVulkan(backend_context.native) })
    }

    // TODO: threadSafeProxy()

    pub fn reset(&mut self, backend_state: Option<u32>) -> &mut Self {
        unsafe {
            self.native_mut().resetContext(backend_state.unwrap_or(skia_bindings::kAll_GrBackendState))
        }
        self
    }

    pub fn abandon(&mut self) -> &mut Self {
        unsafe {
            // self.native_mut().abandonContext()
            GrContext_abandonContext(self.native_mut() as *mut _ as _)
        }
        self
    }

    pub fn abandoned(&self) -> bool {
        unsafe {
            self.native().abandoned()
        }
    }

    pub fn release_resources_and_abandon(&mut self) -> &mut Self {
        unsafe {
            GrContext_releaseResourcesAndAbandonContext(self.native_mut() as *mut _ as _)
        }
        self
    }

    pub fn resource_cache_limits(&self) -> ResourceCacheLimits {
        let mut resources = 0;
        let mut resource_bytes = 0;
        unsafe {
            self.native().getResourceCacheLimits(&mut resources, &mut resource_bytes)
        }
        ResourceCacheLimits {
            max_resources: resources.try_into().unwrap(),
            max_resource_bytes: resource_bytes
        }
    }


    pub fn resource_cache_usage(&self) -> ResourceCacheUsage {
        let mut resource_count = 0;
        let mut resource_bytes = 0;
        unsafe {
            self.native().getResourceCacheUsage(&mut resource_count, &mut resource_bytes)
        }
        ResourceCacheUsage {
            resource_count: resource_count.try_into().unwrap(),
            resource_bytes
        }
    }

    pub fn resource_cache_purgeable_bytes(&self) -> usize {
        unsafe {
            self.native().getResourceCachePurgeableBytes()
        }
    }

    pub fn free_gpu_resources(&mut self) -> &mut Self {
        unsafe {
            GrContext_freeGpuResources(self.native_mut() as *mut _ as _)
        }
        self
    }

    // TODO: performDeferredCleanup()

    pub fn purge_unlocked_resources(&mut self, bytes_to_purge: Option<usize>, prefer_scratch_resources: bool) -> &mut Self {
        unsafe {
            match bytes_to_purge {
                Some(bytes_to_purge) =>
                    self.native_mut().purgeUnlockedResources(bytes_to_purge, prefer_scratch_resources),
                None =>
                    self.native_mut().purgeUnlockedResources1(prefer_scratch_resources)
            }
        }
        self
    }

    pub fn max_texture_size(&self) -> i32 {
        unsafe {
            self.native().maxTextureSize()
        }
    }

    pub fn max_render_target_size(&self) -> i32 {
        unsafe {
            self.native().maxRenderTargetSize()
        }
    }

    pub fn color_type_supported_as_image(&self, color_type: ColorType) -> bool {
        unsafe {
            self.native().colorTypeSupportedAsImage(color_type.into_native())
        }
    }

    pub fn color_type_supported_as_surface(&self, color_type: ColorType) -> bool {
        unsafe {
            // does not link
            // self.native().colorTypeSupportedAsSurface(color_type.into_native())
            skia_bindings::C_GrContext_colorTypeSupportedAsSurface(self.native(), color_type.into_native())
        }
    }

    pub fn max_surface_sample_count_for_color_type(&self, color_type: ColorType) -> usize {
        unsafe {
            self.native().maxSurfaceSampleCountForColorType(color_type.into_native()).try_into().unwrap()
        }
    }

    pub fn flush(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().flush();
        }
        self
    }

    // TODO: flushAndSignalSemaphores

    pub fn unique_id(&mut self) -> u32 {
        unsafe {
            self.native_mut().uniqueID()
        }
    }

    pub fn supports_distance_field_text(&self) -> bool {
        unsafe {
            self.native().supportsDistanceFieldText()
        }
    }

    #[cfg(feature = "vulkan")]
    pub fn store_vulkan_pipeline_cache_data(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().storeVkPipelineCacheDate();
        }
        self
    }
}
