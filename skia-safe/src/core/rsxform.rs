use crate::prelude::*;
use crate::{scalar, Point, Size, Vector};
use skia_bindings::SkRSXform;

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct RSXform {
    pub scos: scalar,
    pub ssin: scalar,
    pub t: Vector,
}

impl NativeTransmutable<SkRSXform> for RSXform {}
#[test]
fn test_rsxform_layout() {
    RSXform::test_layout()
}

impl RSXform {
    pub fn new(scos: scalar, ssin: scalar, t: impl Into<Vector>) -> Self {
        let t = t.into();
        Self { scos, ssin, t }
    }

    pub fn from_radians(
        scale: scalar,
        radians: scalar,
        t: impl Into<Vector>,
        a: impl Into<Point>,
    ) -> Self {
        let t = t.into();
        let a = a.into();

        let s = radians.sin() * scale;
        let c = radians.cos() * scale;
        Self::new(c, s, (t.x + -c * a.x + s * a.y, t.y + -s * a.x - c * a.y))
    }

    pub fn rect_stays_rect(&self) -> bool {
        self.scos == 0.0 || self.ssin == 0.0
    }

    pub fn set_identity(&mut self) {
        self.set(1.0, 0.0, Vector::default())
    }

    pub fn set(&mut self, scos: scalar, ssin: scalar, t: impl Into<Vector>) {
        let t = t.into();
        self.scos = scos;
        self.ssin = ssin;
        self.t = t;
    }

    pub fn to_quad(&self, size: impl Into<Size>) -> [Point; 4] {
        let size = size.into();
        let mut quad: [Point; 4] = Default::default();
        unsafe {
            self.native()
                .toQuad(size.width, size.height, quad.native_mut().as_mut_ptr())
        }
        quad
    }

    pub fn to_tri_strip(&self, size: impl Into<Size>) -> [Point; 4] {
        let size = size.into();
        let mut strip: [Point; 4] = Default::default();
        unsafe {
            self.native()
                .toTriStrip(size.width, size.height, strip.native_mut().as_mut_ptr())
        }
        strip
    }
}
