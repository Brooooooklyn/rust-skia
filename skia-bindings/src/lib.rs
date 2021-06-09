#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]
// GrVkBackendContext contains u128 fields on macOS
#![allow(improper_ctypes)]
// mem::uninitialized()
#![allow(invalid_value)]
#![allow(deprecated)]

include!(concat!(env!("OUT_DIR"), "/skia/bindings.rs"));

mod defaults;
pub use defaults::*;

mod impls;
pub use impls::*;

#[cfg(feature = "textlayout")]
pub mod icu;

#[doc(hidden)]
#[cfg(feature = "use-system-jpeg-turbo")]
use mozjpeg_sys;
