use crate::interop::DynamicMemoryWStream;
use crate::matrix::ApplyPerspectiveClip;
use crate::prelude::*;
use crate::{
    path_types, scalar, Data, Matrix, PathDirection, PathFillType, Point, RRect, Rect, Vector,
};
use skia_bindings as sb;
use skia_bindings::{SkPath, SkPath_Iter, SkPath_RawIter};
use std::marker::PhantomData;
use std::mem::forget;

#[deprecated(since = "0.25.0", note = "use PathDirection")]
pub use path_types::PathDirection as Direction;

#[deprecated(since = "0.25.0", note = "use PathFillType")]
pub use path_types::PathFillType as FillType;

pub use skia_bindings::SkPath_ArcSize as ArcSize;

pub use skia_bindings::SkPath_AddPathMode as AddPathMode;

pub use path_types::PathSegmentMask as SegmentMask;

pub use skia_bindings::SkPath_Verb as Verb;

#[repr(C)]
pub struct Iter<'a>(SkPath_Iter, PhantomData<&'a Handle<SkPath>>);

impl NativeAccess<SkPath_Iter> for Iter<'_> {
    fn native(&self) -> &SkPath_Iter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPath_Iter {
        &mut self.0
    }
}

impl Drop for Iter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_Iter_destruct(&mut self.0) }
    }
}

impl Default for Iter<'_> {
    fn default() -> Self {
        Iter(unsafe { SkPath_Iter::new() }, PhantomData)
    }
}

impl Iter<'_> {
    pub fn new(path: &Path, force_close: bool) -> Iter {
        Iter(
            unsafe { SkPath_Iter::new1(path.native(), force_close) },
            PhantomData,
        )
    }

    pub fn set_path(mut self, path: &Path, force_close: bool) -> Iter {
        unsafe {
            self.0.setPath(path.native(), force_close);
        }
        let r = Iter(self.0, PhantomData);
        forget(self);
        r
    }

    pub fn conic_weight(&self) -> Option<scalar> {
        #[allow(clippy::map_clone)]
        self.native()
            .fConicWeights
            .into_option()
            .map(|p| unsafe { *p })
    }

    pub fn is_close_line(&self) -> bool {
        unsafe { sb::C_SkPath_Iter_isCloseLine(self.native()) }
    }

    pub fn is_closed_contour(&self) -> bool {
        unsafe { self.native().isClosedContour() }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Verb, Vec<Point>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut points = [Point::default(); Verb::MAX_POINTS];
        let verb = unsafe { self.native_mut().next(points.native_mut().as_mut_ptr()) };
        if verb != Verb::Done {
            Some((verb, points[0..verb.points()].into()))
        } else {
            None
        }
    }
}

#[repr(C)]
#[deprecated(
    since = "0.30.0",
    note = "User Iter instead, RawIter will soon be removed."
)]
pub struct RawIter<'a>(SkPath_RawIter, PhantomData<&'a Handle<SkPath>>);

#[allow(deprecated)]
impl NativeAccess<SkPath_RawIter> for RawIter<'_> {
    fn native(&self) -> &SkPath_RawIter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPath_RawIter {
        &mut self.0
    }
}

#[allow(deprecated)]
impl Drop for RawIter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_RawIter_destruct(&mut self.0) }
    }
}

#[allow(deprecated)]
impl Default for RawIter<'_> {
    fn default() -> Self {
        RawIter(
            construct(|ri| unsafe { sb::C_SkPath_RawIter_Construct(ri) }),
            PhantomData,
        )
    }
}

#[allow(deprecated)]
impl RawIter<'_> {
    pub fn new(path: &Path) -> RawIter {
        RawIter::default().set_path(path)
    }

    pub fn set_path(mut self, path: &Path) -> RawIter {
        unsafe { self.native_mut().setPath(path.native()) }
        let r = RawIter(self.0, PhantomData);
        forget(self);
        r
    }

    pub fn peek(&self) -> Verb {
        unsafe { sb::C_SkPath_RawIter_peek(self.native()) }
    }

    pub fn conic_weight(&self) -> scalar {
        self.native().fConicWeight
    }
}

#[allow(deprecated)]
impl Iterator for RawIter<'_> {
    type Item = (Verb, Vec<Point>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut points = [Point::default(); Verb::MAX_POINTS];

        let verb = unsafe { self.native_mut().next(points.native_mut().as_mut_ptr()) };
        (verb != Verb::Done).if_true_some((verb, points[0..verb.points()].into()))
    }
}

/// A Skia shape. This is just the abstract shape, which could be either a fill or a stroke
/// depending on the defined paint (see the documentation for `Paint`). This type is
/// copy-on-write, and so cloning it will share underlying storage until it is mutated.
pub type Path = Handle<SkPath>;

unsafe impl Send for Path {}
unsafe impl Sync for Path {}

impl NativeDrop for SkPath {
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_destruct(self) }
    }
}

impl NativeClone for SkPath {
    fn clone(&self) -> Self {
        unsafe { SkPath::new1(self) }
    }
}

impl NativePartialEq for SkPath {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkPath_Equals(self, rhs) }
    }
}

impl Default for Handle<SkPath> {
    fn default() -> Self {
        Self::new()
    }
}

impl Handle<SkPath> {
    /// Create a path from a set of points and the associated verbs. Verbs are here specified
    /// as bytes, one byte per verb. The `Verb` enum is 32 bits, and you can get the bytes needed
    /// for this function by simply using `foo_verb as u8`.
    ///
    /// Set `fill_type` to choose winding vs even-odd fill mode.
    ///
    /// `is_volatile` selects whether the path is "volatile". A volatile path will never be cached,
    /// a non-volatile path will have intermediate values needed for drawing stored on the path itself
    /// in order to speed up drawing.
    pub fn new_from(
        points: &[Point],
        verbs: &[u8],
        conic_weights: &[scalar],
        fill_type: FillType,
        is_volatile: impl Into<Option<bool>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Make(
                path,
                points.native().as_ptr(),
                points.len().try_into().unwrap(),
                verbs.as_ptr(),
                verbs.len().try_into().unwrap(),
                conic_weights.as_ptr(),
                conic_weights.len().try_into().unwrap(),
                fill_type,
                is_volatile.into().unwrap_or(false),
            )
        })
    }

    /// Create a simple rectangle, optionally speciying winding order.
    pub fn rect(rect: impl AsRef<Rect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Rect(
                path,
                rect.as_ref().native(),
                dir.into().unwrap_or(PathDirection::CW),
            )
        })
    }

    /// Create a simple oval with the position, width and height specified by the input rectangle,
    /// optionally specifying winding order.
    pub fn oval(oval: impl AsRef<Rect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Oval(
                path,
                oval.as_ref().native(),
                dir.into().unwrap_or(PathDirection::CW),
            )
        })
    }

    /// Create a simple oval with the position, width and height specified by the input rectangle,
    /// optionally specifying winding order. An oval is created out of multiple segments, and this
    /// allows you to choose which segment is considered the "first" one.
    pub fn oval_with_start_index(
        oval: impl AsRef<Rect>,
        dir: PathDirection,
        start_index: usize,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_OvalWithStartIndex(
                path,
                oval.as_ref().native(),
                dir,
                start_index.try_into().unwrap(),
            )
        })
    }

    /// Create a circle, optionally specifying winding order.
    pub fn circle(
        center: impl Into<Point>,
        radius: scalar,
        dir: impl Into<Option<PathDirection>>,
    ) -> Self {
        let center = center.into();
        Self::construct(|path| unsafe {
            sb::C_SkPath_Circle(
                path,
                center.x,
                center.y,
                radius,
                dir.into().unwrap_or(PathDirection::CW),
            )
        })
    }

    /// Create a rounded rectangle, optionally specifying winding order (see documentation for `RRect` for
    /// more info).
    pub fn rrect(rect: impl AsRef<RRect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_RRect(
                path,
                rect.as_ref().native(),
                dir.into().unwrap_or(PathDirection::CW),
            )
        })
    }

    /// Create a rounded rectangle, optionally specifying winding order (see documentation for `RRect` for
    /// more info). A rounded rectangle is made out of multiple segments, and this allows you to select
    /// which segment is considered the "first" one.
    pub fn rrect_with_start_index(
        rect: impl AsRef<RRect>,
        dir: PathDirection,
        start_index: usize,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_RRectWithStartIndex(
                path,
                rect.as_ref().native(),
                dir,
                start_index.try_into().unwrap(),
            )
        })
    }

    /// Create a possibly-closed arbitrary polygon with flat sides, from a series of points.
    ///
    /// Set `fill_type` to choose winding vs even-odd fill mode.
    ///
    /// `is_volatile` selects whether the path is "volatile". A volatile path will never be cached,
    /// a non-volatile path will have intermediate values needed for drawing stored on the path itself
    /// in order to speed up drawing.
    pub fn polygon(
        pts: &[Point],
        is_closed: bool,
        fill_type: impl Into<Option<FillType>>,
        is_volatile: impl Into<Option<bool>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Polygon(
                path,
                pts.native().as_ptr(),
                pts.len().try_into().unwrap(),
                is_closed,
                fill_type.into().unwrap_or(FillType::Winding),
                is_volatile.into().unwrap_or(false),
            )
        })
    }

    /// Create a path representing just a single straight line between two points.
    pub fn line(a: impl Into<Point>, b: impl Into<Point>) -> Self {
        Self::polygon(&[a.into(), b.into()], false, None, None)
    }

    /// Create an empty path.
    pub fn new() -> Self {
        Self::construct(|path| unsafe { sb::C_SkPath_Construct(path) })
    }

    /// Returns true if this path can be interpolated with the path specified by `compare`. This is
    /// true if both paths have the same number of segments, and the verb for each segment matches.
    /// If the verb is `Conic`, then the weights must match.
    pub fn is_interpolatable(&self, compare: &Path) -> bool {
        unsafe { self.native().isInterpolatable(compare.native()) }
    }

    /// Create a new path that interpolates between this path and the path specified by `ending`.
    /// `weight` should be between 0 and 1. 0 means that the output will be equal to `self`,
    /// and 1 means that the output will be equal to `ending`. Returns false if the paths are not
    /// interpolatable (see `is_interpolatable`).
    pub fn interpolate(&self, ending: &Path, weight: scalar) -> Option<Path> {
        let mut out = Path::default();
        unsafe {
            self.native()
                .interpolate(ending.native(), weight, out.native_mut())
        }
        .if_true_some(out)
    }

    /// Returns the fill rule for this path (see documentation for `PathFillType`).
    pub fn fill_type(&self) -> PathFillType {
        unsafe { sb::C_SkPath_getFillType(self.native()) }
    }

    /// Sets the fill rule for this path (see documentation for `PathFillType`).
    pub fn set_fill_type(&mut self, ft: PathFillType) -> &mut Self {
        self.native_mut().set_fFillType(ft as _);
        self
    }

    /// Returns true if the fill type is an "inverse" type (see documentation for `PathFillType`).
    pub fn is_inverse_fill_type(&self) -> bool {
        self.fill_type().is_inverse()
    }

    /// Sets the fill type of this path to be the "inverted" equivalent of the current fill type
    /// (see `PathFillType`)
    pub fn toggle_inverse_fill_type(&mut self) -> &mut Self {
        let inverse = self.native().fFillType() ^ 2;
        self.native_mut().set_fFillType(inverse);
        self
    }

    #[deprecated(since = "0.36.0", note = "Removed, use is_convex()")]
    pub fn convexity_type(&self) -> ! {
        panic!("Removed")
    }

    #[deprecated(since = "0.36.0", note = "Removed, use is_convex()")]
    pub fn convexity_type_or_unknown(&self) -> ! {
        panic!("Removed")
    }

    /// Returns true if the path is fully convex.
    pub fn is_convex(&self) -> bool {
        unsafe { sb::C_SkPath_isConvex(self.native()) }
    }

    /// If the path is an oval, returns the bounding rectangle. Otherwise, returns `None`.
    pub fn is_oval(&self) -> Option<Rect> {
        let mut bounds = Rect::default();
        unsafe { self.native().isOval(bounds.native_mut()) }.if_true_some(bounds)
    }

    /// If the path is a rounded rectangle, returns the `RRect` that specifies its shape. Otherwise,
    /// returns `None`.
    pub fn is_rrect(&self) -> Option<RRect> {
        let mut rrect = RRect::default();
        unsafe { self.native().isRRect(rrect.native_mut()) }.if_true_some(rrect)
    }

    /// Clears the path segments, deleting the internal storage and setting all metadata (such as fill
    /// type) to their respective default values.
    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() };
        self
    }

    /// Clears the path segments, setting all metadata (such as fill type) to their respective default
    /// values. Unlike `reset`, this does _not_ delete the internal storage, and so it is useful if you
    /// want to avoid reallocating if you're creating a new path that will have the same number of
    /// segments as an existing path.
    pub fn rewind(&mut self) -> &mut Self {
        unsafe { self.native_mut().rewind() };
        self
    }

    /// Returns true if the path has no segments.
    pub fn is_empty(&self) -> bool {
        unsafe { sb::C_SkPath_isEmpty(self.native()) }
    }

    /// Returns true if the path's verb array was last modified by `fn close`.
    pub fn is_last_contour_closed(&self) -> bool {
        unsafe { self.native().isLastContourClosed() }
    }

    /// Returns true if all the points in this shape have values that are finite. Returns false if
    /// any point has a component that is infinity, negative infinity or NaN.
    pub fn is_finite(&self) -> bool {
        unsafe { sb::C_SkPath_isFinite(self.native()) }
    }

    /// Returns whether this path is volatile or not. A volatile path will not have metadata required
    /// for drawing internally cached. Paths are non-volatile by default.
    pub fn is_volatile(&self) -> bool {
        self.native().fIsVolatile() != 0
    }

    /// Set whether this path is volatile or not. A volatile path will not have metadata required
    /// for drawing internally cached. Paths are non-volatile by default. If you expect to draw a path
    /// multiple times without mutating it then you should leave it as non-volatile. If you are
    /// regenerating the path every time you draw it then you should set the path to be volatile.
    pub fn set_is_volatile(&mut self, is_volatile: bool) -> &mut Self {
        self.native_mut().set_fIsVolatile(is_volatile as _);
        self
    }

    /// Checks whether a line between the two supplied points is "degenerate", i.e. if the points are
    /// close enough that they can be treated as a single point.
    pub fn is_line_degenerate(p1: impl Into<Point>, p2: impl Into<Point>, exact: bool) -> bool {
        unsafe { SkPath::IsLineDegenerate(p1.into().native(), p2.into().native(), exact) }
    }

    /// Checks whether a quadratic bezier curve is degenerate, i.e. if its length is small enough that
    /// it can be treated as a single point.
    pub fn is_quad_degenerate(
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        p3: impl Into<Point>,
        exact: bool,
    ) -> bool {
        unsafe {
            SkPath::IsQuadDegenerate(
                p1.into().native(),
                p2.into().native(),
                p3.into().native(),
                exact,
            )
        }
    }

    /// Checks whether a cubic bezier curve is degenerate, i.e. if its length is small enough that
    /// it can be treated as a single point.
    pub fn is_cubic_degenerate(
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        p3: impl Into<Point>,
        p4: impl Into<Point>,
        exact: bool,
    ) -> bool {
        unsafe {
            SkPath::IsCubicDegenerate(
                p1.into().native(),
                p2.into().native(),
                p3.into().native(),
                p4.into().native(),
                exact,
            )
        }
    }

    /// If this path is a single line, return the start- and endpoints. Otherwise, return `None`.
    pub fn is_line(&self) -> Option<(Point, Point)> {
        let mut line = [Point::default(); 2];
        unsafe { self.native().isLine(line.native_mut().as_mut_ptr()) }
            .if_true_some((line[0], line[1]))
    }

    /// Returns the number of points in this path.
    pub fn count_points(&self) -> usize {
        unsafe { self.native().countPoints().try_into().unwrap() }
    }

    /// Returns the nth point in the path.
    pub fn get_point(&self, index: usize) -> Option<Point> {
        let p = Point::from_native_c(unsafe {
            sb::C_SkPath_getPoint(self.native(), index.try_into().unwrap())
        });
        // assuming that count_points() is somewhat slow, we
        // check the index when a Point(0,0) is returned.
        if p != Point::default() || index < self.count_points() {
            Some(p)
        } else {
            None
        }
    }

    /// Writes the points in this path to the `points` array, returning the number of points that
    /// were written.
    pub fn get_points(&self, points: &mut [Point]) -> usize {
        unsafe {
            self.native().getPoints(
                points.native_mut().as_mut_ptr(),
                points.len().try_into().unwrap(),
            )
        }
        .try_into()
        .unwrap()
    }

    /// Returns the number of verbs in the path.
    pub fn count_verbs(&self) -> usize {
        unsafe { self.native().countVerbs() }.try_into().unwrap()
    }

    /// Writes the verbs in this path to the `verbs` array, returning the number of verbs that were
    /// written.
    pub fn get_verbs(&self, verbs: &mut [u8]) -> usize {
        unsafe {
            self.native()
                .getVerbs(verbs.as_mut_ptr(), verbs.len().try_into().unwrap())
        }
        .try_into()
        .unwrap()
    }

    /// Returns an approximation of the memory used by this path.
    pub fn approximate_bytes_used(&self) -> usize {
        unsafe { self.native().approximateBytesUsed() }
    }

    /// Swaps the data of this path and another path.
    pub fn swap(&mut self, other: &mut Path) -> &mut Self {
        unsafe { self.native_mut().swap(other.native_mut()) }
        self
    }

    /// Returns the approximate bounding box of this path. For paths containing curves, this may be
    /// larger than the tight bounding box, but is far cheaper to calculate. Additionally, the value
    /// is cached. For the tight bounding box, see `compute_tight_bounds`.
    pub fn bounds(&self) -> &Rect {
        Rect::from_native_ref(unsafe { &*sb::C_SkPath_getBounds(self.native()) })
    }

    /// Updates the cached bounding box, if it has become out-of-date.
    pub fn update_bounds_cache(&mut self) -> &mut Self {
        self.bounds();
        self
    }

    /// Calculate the precise bounding box. This is more expensive to compute than the approximate
    /// bounding box, and its value is not cached. However, it is guarateed to be the smallest
    /// rectangle that can contain this path.
    pub fn compute_tight_bounds(&self) -> Rect {
        Rect::from_native_c(unsafe { sb::C_SkPath_computeTightBounds(self.native()) })
    }

    /// Returns true if the rectangle is contained within the outline of this path. This function is
    /// an approximation, and can return false even if the rectangle is contained within this path's
    /// outline. However, it will never return true when the rectangle is _not_ contained within this
    /// path's outline.
    pub fn conservatively_contains_rect(&self, rect: impl AsRef<Rect>) -> bool {
        unsafe {
            self.native()
                .conservativelyContainsRect(rect.as_ref().native())
        }
    }

    /// Reserve space in the verb and point arrays for `extra_pt_count` additional points.
    pub fn inc_reserve(&mut self, extra_pt_count: usize) -> &mut Self {
        unsafe {
            self.native_mut()
                .incReserve(extra_pt_count.try_into().unwrap())
        }
        self
    }

    pub fn shrink_to_fit(&mut self) -> &mut Self {
        unsafe { self.native_mut().shrinkToFit() }
        self
    }

    /// Move the cursor (i.e. the point where the next contour will start) to the point `p`.
    pub fn move_to(&mut self, p: impl Into<Point>) -> &mut Self {
        let p = p.into();
        unsafe {
            self.native_mut().moveTo(p.x, p.y);
        }
        self
    }

    /// Move the cursor (i.e. the point where the next contour will start) by `d`, relative to
    /// the current cursor position.
    pub fn r_move_to(&mut self, d: impl Into<Vector>) -> &mut Self {
        let d = d.into();
        unsafe {
            self.native_mut().rMoveTo(d.x, d.y);
        }
        self
    }

    /// Append a straight line to this path, from the current cursor location to the point `p`.
    pub fn line_to(&mut self, p: impl Into<Point>) -> &mut Self {
        let p = p.into();
        unsafe {
            self.native_mut().lineTo(p.x, p.y);
        }
        self
    }

    /// Append a straight line to this path, from the current cursor location to the point `cursor + d`.
    pub fn r_line_to(&mut self, d: impl Into<Vector>) -> &mut Self {
        let d = d.into();
        unsafe {
            self.native_mut().rLineTo(d.x, d.y);
        }
        self
    }

    /// Append a quadratic bezier curve to this path, from the current cursor location to the point `end`,
    /// with the control point `control`.
    pub fn quad_to(&mut self, control: impl Into<Point>, end: impl Into<Point>) -> &mut Self {
        let p1 = control.into();
        let p2 = end.into();
        unsafe {
            self.native_mut().quadTo(p1.x, p1.y, p2.x, p2.y);
        }
        self
    }

    /// Append a quadratic bezier curve to this path, from the current cursor location to the point
    /// `cursor + end_d`, with the control point `cursor + control_d`.
    pub fn r_quad_to(
        &mut self,
        control_d: impl Into<Vector>,
        end_d: impl Into<Vector>,
    ) -> &mut Self {
        let (dx1, dx2) = (control_d.into(), end_d.into());
        unsafe {
            self.native_mut().rQuadTo(dx1.x, dx1.y, dx2.x, dx2.y);
        }
        self
    }

    /// Append a conic curve to this path.
    pub fn conic_to(&mut self, p1: impl Into<Point>, p2: impl Into<Point>, w: scalar) -> &mut Self {
        let p1 = p1.into();
        let p2 = p2.into();
        unsafe {
            self.native_mut().conicTo(p1.x, p1.y, p2.x, p2.y, w);
        }
        self
    }

    /// Append a conic curve to this path, relative to the current cursor position.
    pub fn r_conic_to(
        &mut self,
        d1: impl Into<Vector>,
        d2: impl Into<Vector>,
        w: scalar,
    ) -> &mut Self {
        let (d1, d2) = (d1.into(), d2.into());
        unsafe {
            self.native_mut().rConicTo(d1.x, d1.y, d2.x, d2.y, w);
        }
        self
    }

    /// Append a cubic bezier curve to this path, from the current cursor location to the point `end`,
    /// with the control points `control1` and `control2`.
    pub fn cubic_to(
        &mut self,
        control1: impl Into<Point>,
        control2: impl Into<Point>,
        end: impl Into<Point>,
    ) -> &mut Self {
        let (p1, p2, p3) = (control1.into(), control2.into(), end.into());
        unsafe {
            self.native_mut()
                .cubicTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
        }
        self
    }

    /// Append a cubic bezier curve to this path, from the current cursor location to the point `cursor + end_d`,
    /// with the control points `cursor + control_d1` and `cursor + control_d2`.
    pub fn r_cubic_to(
        &mut self,
        control_d1: impl Into<Vector>,
        control_d2: impl Into<Vector>,
        end_d: impl Into<Vector>,
    ) -> &mut Self {
        let (d1, d2, d3) = (control_d1.into(), control_d2.into(), end_d.into());
        unsafe {
            self.native_mut()
                .rCubicTo(d1.x, d1.y, d2.x, d2.y, d3.x, d3.y);
        }
        self
    }

    /// Append an arc curve to this path.
    pub fn arc_to(
        &mut self,
        oval: impl AsRef<Rect>,
        start_angle: scalar,
        sweep_angle: scalar,
        force_move_to: bool,
    ) -> &mut Self {
        unsafe {
            self.native_mut().arcTo(
                oval.as_ref().native(),
                start_angle,
                sweep_angle,
                force_move_to,
            );
        }
        self
    }

    /// Append an arc curve to this path, bounded by the triangle defined by the cursor, p1 and p2. The
    /// arc is a component of a circle with the specified radius, positioned such that it touches both
    /// tangent lines.
    pub fn arc_to_tangent(
        &mut self,
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        radius: scalar,
    ) -> &mut Self {
        let (p1, p2) = (p1.into(), p2.into());
        unsafe {
            self.native_mut().arcTo1(p1.x, p1.y, p2.x, p2.y, radius);
        }
        self
    }

    /// Append an arc curve to this path, that is part of the oval defined by xy rotated by `x_axis_rotate`
    /// (specified in degrees). The `PathDirection` specifies whether it is clockwise or anticlockwise and
    /// `large_arc` specifies whether the smaller or larger of the two arc lengths is chosen. The arc ends
    /// at `xy`.
    pub fn arc_to_rotated(
        &mut self,
        r: impl Into<Point>,
        x_axis_rotate: scalar,
        large_arc: ArcSize,
        sweep: PathDirection,
        xy: impl Into<Point>,
    ) -> &mut Self {
        let (r, xy) = (r.into(), xy.into());
        unsafe {
            self.native_mut()
                .arcTo2(r.x, r.y, x_axis_rotate, large_arc, sweep, xy.x, xy.y);
        }
        self
    }

    /// Append an arc curve to this path, that is part of the oval defined by xy rotated by `x_axis_rotate`
    /// (specified in degrees). The `PathDirection` specifies whether it is clockwise or anticlockwise and
    /// `large_arc` specifies whether the smaller or larger of the two arc lengths is chosen. The arc ends
    /// at `cursor + xy`.
    pub fn r_arc_to_rotated(
        &mut self,
        r: impl Into<Point>,
        x_axis_rotate: scalar,
        large_arc: ArcSize,
        sweep: PathDirection,
        xy: impl Into<Point>,
    ) -> &mut Self {
        let (r, xy) = (r.into(), xy.into());
        unsafe {
            self.native_mut()
                .rArcTo(r.x, r.y, x_axis_rotate, large_arc, sweep, xy.x, xy.y);
        }
        self
    }

    /// Close the path, appending a line between the first and last point of the path.
    pub fn close(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().close();
        }
        self
    }

    /// Converts a conic curve to a series of quads and writes them to `pts`.
    pub fn convert_conic_to_quads(
        p0: impl Into<Point>,
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        w: scalar,
        pts: &mut [Point],
        pow2: usize,
    ) -> Option<usize> {
        let (p0, p1, p2) = (p0.into(), p1.into(), p2.into());
        let max_pts_count = 1 + 2 * (1 << pow2);
        if pts.len() >= max_pts_count {
            Some(unsafe {
                SkPath::ConvertConicToQuads(
                    p0.native(),
                    p1.native(),
                    p2.native(),
                    w,
                    pts.native_mut().as_mut_ptr(),
                    pow2.try_into().unwrap(),
                )
                .try_into()
                .unwrap()
            })
        } else {
            None
        }
    }

    // TODO: return type is probably worth a struct.
    /// If the path is equivalent to a rectangle when filled, returns a tuple with the rectangle, whether
    /// the rectangle is closed, and the direction of the path. Otherwise, returns `None`.
    pub fn is_rect(&self) -> Option<(Rect, bool, PathDirection)> {
        let mut rect = Rect::default();
        let mut is_closed = Default::default();
        let mut direction = PathDirection::default();
        unsafe {
            self.native()
                .isRect(rect.native_mut(), &mut is_closed, &mut direction)
        }
        .if_true_some((rect, is_closed, direction))
    }

    /// Appends a rectangle to the current path. See `fn rect`.
    pub fn add_rect(
        &mut self,
        rect: impl AsRef<Rect>,
        dir_start: Option<(PathDirection, usize)>,
    ) -> &mut Self {
        let dir = dir_start.map(|ds| ds.0).unwrap_or_default();
        let start = dir_start.map(|ds| ds.1).unwrap_or_default();
        unsafe {
            self.native_mut()
                .addRect1(rect.as_ref().native(), dir, start.try_into().unwrap())
        };
        self
    }

    /// Appends an oval to the current path. See `fn oval` and `fn oval_with_start_index`.
    pub fn add_oval(
        &mut self,
        oval: impl AsRef<Rect>,
        dir_start: Option<(PathDirection, usize)>,
    ) -> &mut Self {
        let dir = dir_start.map(|ds| ds.0).unwrap_or_default();
        let start = dir_start.map(|ds| ds.1).unwrap_or_default();
        unsafe {
            self.native_mut()
                .addOval1(oval.as_ref().native(), dir, start.try_into().unwrap())
        };
        self
    }

    /// Appends a circle to the current path. See `fn circle` and `fn circle_with_start_index`.
    pub fn add_circle(
        &mut self,
        p: impl Into<Point>,
        radius: scalar,
        dir: impl Into<Option<PathDirection>>,
    ) -> &mut Self {
        let p = p.into();
        let dir = dir.into().unwrap_or_default();
        unsafe { self.native_mut().addCircle(p.x, p.y, radius, dir) };
        self
    }

    /// Appends an arc to the path, that is the segment between `start_angle` and `start_angle + sweep_angle`
    /// degrees of the oval specified by `oval`, starting at the current cursor position.
    pub fn add_arc(
        &mut self,
        oval: impl AsRef<Rect>,
        start_angle: scalar,
        sweep_angle: scalar,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .addArc(oval.as_ref().native(), start_angle, sweep_angle)
        };
        self
    }

    // decided to only provide the simpler variant of the two, if radii needs to be specified,
    // add_rrect can be used.
    /// Simple helper function for `add_rrect` that takes a rectangle and the corner radii.
    pub fn add_round_rect(
        &mut self,
        rect: impl AsRef<Rect>,
        (rx, ry): (scalar, scalar),
        dir: impl Into<Option<PathDirection>>,
    ) -> &mut Self {
        let dir = dir.into().unwrap_or_default();
        unsafe {
            self.native_mut()
                .addRoundRect(rect.as_ref().native(), rx, ry, dir)
        };
        self
    }

    /// Append a rounded rectangle to the path. See `fn rrect` and `fn rrect_with_start_index`.
    pub fn add_rrect(
        &mut self,
        rrect: impl AsRef<RRect>,
        dir_start: Option<(PathDirection, usize)>,
    ) -> &mut Self {
        let dir = dir_start.map(|ds| ds.0).unwrap_or_default();
        let start = dir_start.map(|ds| ds.1).unwrap_or_default();
        unsafe {
            self.native_mut()
                .addRRect1(rrect.as_ref().native(), dir, start.try_into().unwrap())
        };
        self
    }

    /// Append a polygon to the path. See `fn polygon`.
    pub fn add_poly(&mut self, pts: &[Point], close: bool) -> &mut Self {
        unsafe {
            self.native_mut()
                .addPoly(pts.native().as_ptr(), pts.len().try_into().unwrap(), close)
        };
        self
    }

    // TODO: addPoly(initializer_list)

    /// Combine this path with `src`, with positions in `src` offset by the vector `d`.
    /// `AddPathMode::Append` combines the shapes without connecting them, `AddPathMode::Extend`
    /// connects the current cursor to the start of the `src` path if the current contour is not closed.
    pub fn add_path(
        &mut self,
        src: &Path,
        d: impl Into<Vector>,
        mode: impl Into<Option<AddPathMode>>,
    ) -> &mut Self {
        let d = d.into();
        let mode = mode.into().unwrap_or(AddPathMode::Append);
        unsafe { self.native_mut().addPath(src.native(), d.x, d.y, mode) };
        self
    }

    // TODO: rename to add_path_with_matrix() ?
    /// Combine this path with `src`, with positions in `src` transformed by the matrix `matrix`.
    /// `AddPathMode::Append` combines the shapes without connecting them, `AddPathMode::Extend`
    /// connects the current cursor to the start of the `src` path if the current contour is not closed.
    pub fn add_path_matrix(
        &mut self,
        src: &Path,
        matrix: &Matrix,
        mode: impl Into<Option<AddPathMode>>,
    ) -> &mut Self {
        let mode = mode.into().unwrap_or(AddPathMode::Append);
        unsafe {
            self.native_mut()
                .addPath1(src.native(), matrix.native(), mode)
        };
        self
    }

    /// Combine this path with `src`, from end to start, with positions in `src` offset by the vector `d`.
    /// This is always treated as `AddPathMode::Extend` (i.e. always connects the current cursor with the
    /// end point of `src`).
    pub fn reverse_add_path(&mut self, src: &Path) -> &mut Self {
        unsafe { self.native_mut().reverseAddPath(src.native()) };
        self
    }

    /// Create a new path that is the same as this one but with all points offset by the given vector `d`.
    #[must_use]
    pub fn with_offset(&self, d: impl Into<Vector>) -> Path {
        let d = d.into();
        let mut path = Path::default();
        unsafe { self.native().offset(d.x, d.y, path.native_mut()) };
        path
    }

    /// Move this path by vector `d`.
    pub fn offset(&mut self, d: impl Into<Vector>) -> &mut Self {
        let d = d.into();
        let self_ptr = self.native_mut() as *mut _;
        unsafe { self.native().offset(d.x, d.y, self_ptr) };
        self
    }

    /// Create a new path that is the same as this one but transformed by the matrix `matrix`.
    ///
    /// If the matrix has a bottom row other than `0, 0, 1` (i.e. if it transforms the `w` component of the
    /// path's points) then the points will be perspective clipped, which avoids dividing by zero or returning
    /// negative values for `w`. This is usually what you want as it avoids confusing results, but if you
    /// want to disable this behavior see `fn with_transform_with_perspective_clip`.
    #[must_use]
    pub fn with_transform(&self, matrix: &Matrix) -> Path {
        self.with_transform_with_perspective_clip(matrix, ApplyPerspectiveClip::Yes)
    }

    /// Create a new path that is the same as this one but transformed by the matrix `matrix`.
    ///
    /// In the case that `perspective_clip` is `ApplyPerspectiveClip::Yes`: If the matrix has a bottom row other
    /// than `0, 0, 1` (i.e. if it transforms the `w` component of the path's points) then the points will
    /// be perspective clipped, which avoids dividing by zero or returning negative values for `w`. This is
    /// usually what you want as it avoids confusing results.
    ///
    /// In the case that `perspective_clip` is `ApplyPerspectiveClip::No`: The math is applied in a "brute force"
    /// manner, meaning that strange results may occur when the matrix has a bottom row that transforms the `w`
    /// component.
    pub fn with_transform_with_perspective_clip(
        &self,
        matrix: &Matrix,
        perspective_clip: ApplyPerspectiveClip,
    ) -> Path {
        let mut path = Path::default();
        unsafe {
            self.native()
                .transform(matrix.native(), path.native_mut(), perspective_clip)
        };
        path
    }

    /// Transform this path in-place by the matrix `matrix`.
    ///
    /// If the matrix has a bottom row other than `0, 0, 1` (i.e. if it transforms the `w` component of the
    /// path's points) then the points will be perspective clipped, which avoids dividing by zero or returning
    /// negative values for `w`. This is usually what you want as it avoids confusing results, but if you
    /// want to disable this behavior see `fn transform_with_perspective_clip`.
    pub fn transform(&mut self, matrix: &Matrix) -> &mut Self {
        self.transform_with_perspective_clip(matrix, ApplyPerspectiveClip::Yes)
    }

    /// Transform this path in-place by the matrix `matrix`.
    ///
    /// In the case that `perspective_clip` is `ApplyPerspectiveClip::Yes`: If the matrix has a bottom row other
    /// than `0, 0, 1` (i.e. if it transforms the `w` component of the path's points) then the points will
    /// be perspective clipped, which avoids dividing by zero or returning negative values for `w`. This is
    /// usually what you want as it avoids confusing results.
    ///
    /// In the case that `perspective_clip` is `ApplyPerspectiveClip::No`: The math is applied in a "brute force"
    /// manner, meaning that strange results may occur when the matrix has a bottom row that transforms the `w`
    /// component.
    pub fn transform_with_perspective_clip(
        &mut self,
        matrix: &Matrix,
        perspective_clip: ApplyPerspectiveClip,
    ) -> &mut Self {
        let self_ptr = self.native_mut() as *mut _;
        unsafe {
            self.native()
                .transform(matrix.native(), self_ptr, perspective_clip)
        };
        self
    }

    /// Get the last point that the path ends at, if the path is non-empty and the last contour is not closed.
    /// This is the cursor that is used when adding new contours to the path.
    pub fn last_pt(&self) -> Option<Point> {
        let mut last_pt = Point::default();
        unsafe { self.native().getLastPt(last_pt.native_mut()) }.if_true_some(last_pt)
    }

    /// See `with_transform_with_perspective_clip`.
    pub fn make_transform(
        &mut self,
        m: &Matrix,
        pc: impl Into<Option<ApplyPerspectiveClip>>,
    ) -> Path {
        self.with_transform_with_perspective_clip(
            &m,
            pc.into().unwrap_or(ApplyPerspectiveClip::Yes),
        )
    }

    /// Create a new path that is the same as this one, but scaled by `sx` in the x direction and `sy` in the
    /// y direction, around the point (0, 0).
    pub fn make_scale(&mut self, (sx, sy): (scalar, scalar)) -> Path {
        self.make_transform(&Matrix::scale((sx, sy)), ApplyPerspectiveClip::No)
    }

    /// Set the cursor position that will be used when adding new components to this path. You probably
    /// want to use `move_to` instead.
    pub fn set_last_pt(&mut self, p: impl Into<Point>) -> &mut Self {
        let p = p.into();
        unsafe { self.native_mut().setLastPt(p.x, p.y) };
        self
    }

    /// Returns a bitset where the corresponding bit is set if the path contains one or more components of
    /// that type.
    pub fn segment_masks(&self) -> SegmentMask {
        SegmentMask::from_bits_truncate(unsafe { sb::C_SkPath_getSegmentMasks(self.native()) })
    }

    /// Returns true if the point `p` is within the path's outline.
    pub fn contains(&self, p: impl Into<Point>) -> bool {
        let p = p.into();
        unsafe { self.native().contains(p.x, p.y) }
    }

    /// Write this path out to a value of type `SkData`. This is for debugging purposes, if you want to write
    /// the path out in a format that can be read back later, you should use `fn serialize`.
    pub fn dump_as_data(&self, force_close: bool, dump_as_hex: bool) -> Data {
        let mut stream = DynamicMemoryWStream::new();
        unsafe {
            self.native()
                .dump(stream.native_mut().base_mut(), force_close, dump_as_hex);
        }
        stream.detach_as_data()
    }

    pub fn dump(&self) {
        unsafe { self.native().dump1() }
    }

    pub fn dump_hex(&self) {
        unsafe { self.native().dumpHex() }
    }

    // TODO: writeToMemory()?

    /// Write this path to memory in a format that can be deserialized by the same version of Skia. The format
    /// is unspecified and not guaranteed to be compatible across versions.
    pub fn serialize(&self) -> Data {
        Data::from_ptr(unsafe { sb::C_SkPath_serialize(self.native()) }).unwrap()
    }

    // TODO: readFromMemory()?

    /// Read this path from data in memory. The format that this function reads is unspecified, other than that
    /// it can read the format produced by the `Path::serialize` function in the same version of Skia.
    pub fn deserialize(data: &Data) -> Option<Path> {
        let mut path = Path::default();
        let bytes = data.as_bytes();
        unsafe {
            path.native_mut()
                .readFromMemory(bytes.as_ptr() as _, bytes.len())
                > 0
        }
        .if_true_some(path)
    }

    /// Return a non-zero, globally-unique identifier that represents this path and compares equal between two
    /// identical paths and non-equal otherwise. It takes into account verbs, points and conic weights, but not
    /// fill type.
    pub fn generation_id(&self) -> u32 {
        unsafe { self.native().getGenerationID() }
    }

    /// Returns true if the data in the path is consistent. Returns false if internal values are out of range, or if
    /// internal storage has an unexpected size.
    pub fn is_valid(&self) -> bool {
        unsafe { sb::C_SkPath_isValid(self.native()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{AddPathMode, ArcSize, Path, PathFillType, Point, Rect, Verb};

    #[test]
    pub fn test_verb_naming() {
        let _ = Verb::Line;
    }

    #[test]
    fn test_arc_size_naming() {
        let _ = ArcSize::Small;
    }

    #[test]
    fn test_add_path_mode_naming() {
        let _ = AddPathMode::Append;
    }

    #[test]
    fn test_get_points() {
        let mut p = Path::new();
        p.add_rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);
        let points_count = p.count_points();
        let mut points = vec![Point::default(); points_count];
        let count_returned = p.get_points(&mut points);
        assert_eq!(count_returned, points.len());
        assert_eq!(count_returned, 4);
    }

    #[test]
    fn test_fill_type() {
        let mut p = Path::default();
        assert_eq!(p.fill_type(), PathFillType::Winding);
        p.set_fill_type(PathFillType::EvenOdd);
        assert_eq!(p.fill_type(), PathFillType::EvenOdd);
        assert!(!p.is_inverse_fill_type());
        p.toggle_inverse_fill_type();
        assert_eq!(p.fill_type(), PathFillType::InverseEvenOdd);
        assert!(p.is_inverse_fill_type());
    }

    #[test]
    fn test_is_volatile() {
        let mut p = Path::default();
        assert!(!p.is_volatile());
        p.set_is_volatile(true);
        assert!(p.is_volatile());
    }

    #[test]
    fn test_path_rect() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let path = Path::rect(r, None);
        assert_eq!(*path.bounds(), r);
    }
}
