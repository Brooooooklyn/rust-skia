use crate::prelude::*;
use crate::{scalar, Matrix, NativeFlattenable, Path, Point, Rect, StrokeRec, Vector};
use skia_bindings as sb;
use skia_bindings::{
    SkFlattenable, SkPathEffect, SkPathEffect_DashType, SkPathEffect_PointData, SkRefCntBase,
};
use std::{fmt, os::raw};

#[repr(C)]
#[derive(Debug)]
pub struct PointData {
    pub flags: point_data::PointFlags,
    points: *const Point,
    num_points: raw::c_int,
    pub size: Vector,
    pub clip_rect: Rect,
    pub path: Path,
    pub first: Path,
    pub last: Path,
}

unsafe impl Send for PointData {}
unsafe impl Sync for PointData {}

impl NativeTransmutable<SkPathEffect_PointData> for PointData {}

#[test]
fn test_point_data_layout() {
    Point::test_layout();
    Vector::test_layout();
    Rect::test_layout();
    PointData::test_layout();
}

impl Drop for PointData {
    fn drop(&mut self) {
        unsafe {
            // we can't call destruct, because it would destruct
            // other fields like Path, which would also be dropped individually,
            // so we just delete the points array here.
            sb::C_SkPathEffect_PointData_deletePoints(self.native_mut())
        }
    }
}

impl Default for PointData {
    fn default() -> Self {
        PointData::construct(|point_data| unsafe {
            sb::C_SkPathEffect_PointData_Construct(point_data)
        })
    }
}

impl PointData {
    pub fn points(&self) -> &[Point] {
        unsafe { safer::from_raw_parts(self.points, self.num_points.try_into().unwrap()) }
    }
}

pub mod point_data {
    use skia_bindings as sb;

    bitflags! {
        pub struct PointFlags: u32 {
            const CIRCLES = sb::SkPathEffect_PointData_PointFlags_kCircles_PointFlag as _;
            const USE_PATH = sb::SkPathEffect_PointData_PointFlags_kUsePath_PointFlag as _;
            const USE_CLIP = sb::SkPathEffect_PointData_PointFlags_kUseClip_PointFlag as _;
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct DashInfo {
    pub intervals: Vec<scalar>,
    pub phase: scalar,
}

pub type PathEffect = RCHandle<SkPathEffect>;
unsafe impl Send for PathEffect {}
unsafe impl Sync for PathEffect {}

impl NativeBase<SkRefCntBase> for SkPathEffect {}
impl NativeBase<SkFlattenable> for SkPathEffect {}

impl NativeRefCountedBase for SkPathEffect {
    type Base = SkRefCntBase;
}

impl NativeFlattenable for SkPathEffect {
    fn native_flattenable(&self) -> &SkFlattenable {
        self.base()
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkPathEffect_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl fmt::Debug for PathEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathEffect")
            .field("as_a_dash", &self.as_a_dash())
            .finish()
    }
}

impl PathEffect {
    pub fn sum(first: impl Into<PathEffect>, second: impl Into<PathEffect>) -> PathEffect {
        PathEffect::from_ptr(unsafe {
            sb::C_SkPathEffect_MakeSum(first.into().into_ptr(), second.into().into_ptr())
        })
        .unwrap()
    }

    pub fn compose(first: impl Into<PathEffect>, second: impl Into<PathEffect>) -> PathEffect {
        PathEffect::from_ptr(unsafe {
            sb::C_SkPathEffect_MakeCompose(first.into().into_ptr(), second.into().into_ptr())
        })
        .unwrap()
    }

    pub fn filter_path(
        &self,
        src: &Path,
        stroke_rec: &StrokeRec,
        cull_rect: impl AsRef<Rect>,
    ) -> Option<(Path, StrokeRec)> {
        let mut dst = Path::default();
        let mut stroke_rec_r = stroke_rec.clone();
        self.filter_path_inplace(&mut dst, src, &mut stroke_rec_r, cull_rect)
            .if_true_some((dst, stroke_rec_r))
    }

    pub fn filter_path_inplace(
        &self,
        dst: &mut Path,
        src: &Path,
        stroke_rec: &mut StrokeRec,
        cull_rect: impl AsRef<Rect>,
    ) -> bool {
        unsafe {
            self.native().filterPath(
                dst.native_mut(),
                src.native(),
                stroke_rec.native_mut(),
                cull_rect.as_ref().native(),
            )
        }
    }

    // TODO: rename to to_points()?
    pub fn as_points(
        &self,
        src: &Path,
        stroke_rect: &StrokeRec,
        matrix: &Matrix,
        cull_rect: impl AsRef<Rect>,
    ) -> Option<PointData> {
        let mut point_data = PointData::default();
        unsafe {
            self.native().asPoints(
                point_data.native_mut(),
                src.native(),
                stroke_rect.native(),
                matrix.native(),
                cull_rect.as_ref().native(),
            )
        }
        .if_true_some(point_data)
    }

    // TODO: rename to to_a_dash()?
    pub fn as_a_dash(&self) -> Option<DashInfo> {
        let mut dash_info = construct(|di| unsafe { sb::C_SkPathEffect_DashInfo_Construct(di) });

        let dash_type = unsafe { self.native().asADash(&mut dash_info) };

        match dash_type {
            SkPathEffect_DashType::kDash_DashType => {
                let mut v: Vec<scalar> = vec![0.0; dash_info.fCount.try_into().unwrap()];
                dash_info.fIntervals = v.as_mut_ptr();
                unsafe {
                    assert_eq!(dash_type, self.native().asADash(&mut dash_info));
                }
                Some(DashInfo {
                    intervals: v,
                    phase: dash_info.fPhase,
                })
            }
            SkPathEffect_DashType::kNone_DashType => None,
        }
    }
}

#[test]
fn create_and_drop_point_data() {
    let data = PointData::default();
    drop(data)
}
