use super::{ID3D12Resource, D3D12_RESOURCE_STATES, DXGI_FORMAT};
use crate::{gpu, prelude::*};
use skia_bindings::{GrD3DAlloc, GrD3DMemoryAllocator, GrD3DTextureResourceInfo, SkRefCntBase};
use std::fmt;
use winapi::{
    shared::dxgiformat,
    shared::dxgitype,
    um::{d3d12, unknwnbase::IUnknown},
};

pub use wio::com::ComPtr as cp;

pub fn safe_com_add_ref<T: winapi::Interface>(ptr: *mut T) -> *mut T {
    if !ptr.is_null() {
        unsafe { (*(ptr as *mut IUnknown)).AddRef() };
    }
    ptr
}

pub fn safe_com_release<T: winapi::Interface>(ptr: *mut T) {
    if !ptr.is_null() {
        unsafe { (*(ptr as *mut IUnknown)).Release() };
    }
}

impl<T> NativeTransmutable<skia_bindings::gr_cp<T>> for cp<T> {}
#[test]
fn test_cp_layout() {
    cp::<super::ID3D12Device>::test_layout();
}

// TODO: add remaining cp functions to ComPtr via traits (get, reset, retain).

pub type Alloc = RCHandle<GrD3DAlloc>;
unsafe impl Send for Alloc {}
unsafe impl Sync for Alloc {}

impl NativeRefCountedBase for GrD3DAlloc {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Alloc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Alloc").finish()
    }
}

// TODO: support the implementation of custom D3D memory allocator's
// virtual createResource() and createAliasingResource() functions.
pub type MemoryAllocator = RCHandle<GrD3DMemoryAllocator>;
unsafe impl Send for MemoryAllocator {}
unsafe impl Sync for MemoryAllocator {}

impl NativeRefCountedBase for GrD3DMemoryAllocator {
    type Base = SkRefCntBase;
}

impl fmt::Debug for MemoryAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryAllocator").finish()
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct TextureResourceInfo {
    pub resource: cp<ID3D12Resource>,
    pub alloc: Option<Alloc>,
    pub resource_state: D3D12_RESOURCE_STATES,
    pub format: DXGI_FORMAT,
    pub sample_count: u32,
    pub level_count: u32,
    pub sample_quality_pattern: std::os::raw::c_uint,
    pub protected: gpu::Protected,
}
unsafe impl Send for TextureResourceInfo {}
unsafe impl Sync for TextureResourceInfo {}

impl TextureResourceInfo {
    pub fn from_resource(resource: cp<ID3D12Resource>) -> Self {
        Self {
            resource,
            alloc: None,
            resource_state: d3d12::D3D12_RESOURCE_STATE_COMMON,
            format: dxgiformat::DXGI_FORMAT_UNKNOWN,
            sample_count: 1,
            level_count: 0,
            sample_quality_pattern: dxgitype::DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN,
            protected: gpu::Protected::No,
        }
    }

    pub fn with_state(self, resource_state: D3D12_RESOURCE_STATES) -> Self {
        Self {
            resource_state,
            ..self
        }
    }
}

impl From<cp<ID3D12Resource>> for TextureResourceInfo {
    fn from(resource: cp<ID3D12Resource>) -> Self {
        Self::from_resource(resource)
    }
}

impl NativeTransmutable<GrD3DTextureResourceInfo> for TextureResourceInfo {}
#[test]
fn test_texture_resource_info_layout() {
    TextureResourceInfo::test_layout();
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FenceInfo {
    pub fence: cp<d3d12::ID3D12Fence>,
    pub value: u64,
}
unsafe impl Send for FenceInfo {}
unsafe impl Sync for FenceInfo {}
