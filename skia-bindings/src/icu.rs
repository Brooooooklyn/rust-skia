#[cfg(windows)]
pub fn init() {
    use std::env;
    let icudtl = include_bytes!(concat!(env!("OUT_DIR"), "/skia/icudtl.dat"));
    unsafe { crate::C_SetICU(&icudtl[0] as &'static u8 as *const u8 as _) };
}

#[cfg(not(windows))]
pub fn init() {}
