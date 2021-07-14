use skia_bindings::{
    C_SkRefCntBase_ref, C_SkRefCntBase_unique, C_SkRefCntBase_unref, SkNVRefCnt, SkRefCnt,
    SkRefCntBase,
};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr, slice,
};

// Re-export TryFrom / TryInto to make them available in all modules that use prelude::*.
pub use std::convert::{TryFrom, TryInto};

/// Swiss army knife to convert any reference into any other.
pub(crate) unsafe fn transmute_ref<FromT, ToT>(from: &FromT) -> &ToT {
    // TODO: can we do this statically for all instantiations of transmute_ref?
    debug_assert_eq!(mem::size_of::<FromT>(), mem::size_of::<ToT>());
    &*(from as *const FromT as *const ToT)
}

pub(crate) unsafe fn transmute_ref_mut<FromT, ToT>(from: &mut FromT) -> &mut ToT {
    // TODO: can we do this statically for all instantiations of transmute_ref_mut?
    debug_assert_eq!(mem::size_of::<FromT>(), mem::size_of::<ToT>());
    &mut *(from as *mut FromT as *mut ToT)
}

pub(crate) trait IntoOption {
    type Target;
    fn into_option(self) -> Option<Self::Target>;
}

impl<T> IntoOption for *const T {
    type Target = *const T;

    fn into_option(self) -> Option<Self::Target> {
        if !self.is_null() {
            Some(self)
        } else {
            None
        }
    }
}

impl<T> IntoOption for *mut T {
    type Target = ptr::NonNull<T>;

    fn into_option(self) -> Option<Self::Target> {
        ptr::NonNull::new(self)
    }
}

impl IntoOption for bool {
    type Target = ();

    fn into_option(self) -> Option<Self::Target> {
        if self {
            Some(())
        } else {
            None
        }
    }
}

pub(crate) trait IfBoolSome {
    fn if_true_some<V>(self, v: V) -> Option<V>;
    fn if_false_some<V>(self, v: V) -> Option<V>;
    fn if_true_then_some<V>(self, f: impl FnOnce() -> V) -> Option<V>;
    fn if_false_then_some<V>(self, f: impl FnOnce() -> V) -> Option<V>;
}

impl IfBoolSome for bool {
    fn if_true_some<V>(self, v: V) -> Option<V> {
        self.into_option().and(Some(v))
    }

    fn if_false_some<V>(self, v: V) -> Option<V> {
        (!self).if_true_some(v)
    }

    fn if_true_then_some<V>(self, f: impl FnOnce() -> V) -> Option<V> {
        self.into_option().map(|()| f())
    }

    fn if_false_then_some<V>(self, f: impl FnOnce() -> V) -> Option<V> {
        (!self).into_option().map(|()| f())
    }
}

pub(crate) trait RefCount {
    fn ref_cnt(&self) -> usize;
}

impl RefCount for SkRefCntBase {
    // the problem here is that the binding generator represents std::atomic as an u8 (we
    // are lucky that the C alignment rules make space for an i32), so to get the ref
    // counter, we need to get the u8 pointer to fRefCnt and interpret it as an i32 pointer.
    #[allow(clippy::cast_ptr_alignment)]
    fn ref_cnt(&self) -> usize {
        unsafe {
            let ptr: *const i32 = &self.fRefCnt as *const _ as *const i32;
            (*ptr).try_into().unwrap()
        }
    }
}

impl NativeBase<SkRefCntBase> for SkRefCnt {}

impl RefCount for SkRefCnt {
    fn ref_cnt(&self) -> usize {
        self.base().ref_cnt()
    }
}

impl RefCount for SkNVRefCnt {
    #[allow(clippy::cast_ptr_alignment)]
    fn ref_cnt(&self) -> usize {
        unsafe {
            let ptr: *const i32 = &self.fRefCnt as *const _ as *const i32;
            (*ptr).try_into().unwrap()
        }
    }
}

pub trait NativeRefCounted: Sized {
    fn _ref(&self);
    fn _unref(&self);
    fn unique(&self) -> bool;
    fn _ref_cnt(&self) -> usize {
        unimplemented!();
    }
}

impl NativeRefCounted for SkRefCntBase {
    fn _ref(&self) {
        unsafe { C_SkRefCntBase_ref(self) }
    }

    fn _unref(&self) {
        unsafe { C_SkRefCntBase_unref(self) }
    }

    fn unique(&self) -> bool {
        unsafe { C_SkRefCntBase_unique(self) }
    }

    #[allow(clippy::cast_ptr_alignment)]
    fn _ref_cnt(&self) -> usize {
        unsafe {
            let ptr: *const i32 = &self.fRefCnt as *const _ as *const i32;
            (*ptr).try_into().unwrap()
        }
    }
}

/// Implements NativeRefCounted by just providing a reference to the base class
/// that implements a RefCount.
/// TODO: use NativeBase
pub trait NativeRefCountedBase {
    type Base: NativeRefCounted;

    /// Returns the ref counter base class of the ref counted type.
    ///
    /// Default implementation assumes that the base class ptr is the same as the
    /// ptr to self.
    fn ref_counted_base(&self) -> &Self::Base {
        unsafe { &*(self as *const _ as *const Self::Base) }
    }
}

impl<Native, Base: NativeRefCounted> NativeRefCounted for Native
where
    Native: NativeRefCountedBase<Base = Base>,
{
    fn _ref(&self) {
        self.ref_counted_base()._ref();
    }

    fn _unref(&self) {
        self.ref_counted_base()._unref();
    }

    fn unique(&self) -> bool {
        self.ref_counted_base().unique()
    }

    fn _ref_cnt(&self) -> usize {
        self.ref_counted_base()._ref_cnt()
    }
}

/// Trait that enables access to a native representation of a wrapper type.
pub trait NativeAccess<N> {
    /// Provides shared access to the native type of the wrapper.
    fn native(&self) -> &N;

    /// Provides exclusive access to the native type of the wrapper.
    fn native_mut(&mut self) -> &mut N;

    // Returns a ptr to the native mutable value.
    unsafe fn native_mut_force(&self) -> *mut N {
        self.native() as *const N as *mut N
    }
}

/// Implements Drop for native types we can not implement Drop for.
pub trait NativeDrop {
    fn drop(&mut self);
}

/// Clone for bindings types we can not implement Clone for.
pub trait NativeClone {
    fn clone(&self) -> Self;
}

/// Even though some types may have value semantics, equality
/// comparison may need to be customized.
pub trait NativePartialEq {
    fn eq(&self, rhs: &Self) -> bool;
}

/// Implements Hash for the native type so that the wrapper type
/// can derive it from.
pub trait NativeHash {
    fn hash<H: Hasher>(&self, state: &mut H);
}

/// Wraps a native type that can be represented and used in Rust memory.
///
/// This type requires the trait `NativeDrop` to be implemented.
#[repr(transparent)]
pub struct Handle<N: NativeDrop>(
    N,
    // `*const` is needed to suppress automatic Send and Sync derivation, which happens when the
    // underlying type generated by bindgen is Send and Sync.
    PhantomData<*const ()>,
);

impl<N: NativeDrop> AsRef<Handle<N>> for Handle<N> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<N: NativeDrop> Handle<N> {
    /// Wrap a native instance into a handle.
    pub(crate) fn from_native_c(n: N) -> Self {
        Handle(n, PhantomData)
    }

    /// Create a reference to the Rust wrapper from a reference to the native type.
    pub(crate) fn from_native_ref(n: &N) -> &Self {
        unsafe { transmute_ref(n) }
    }

    /// Create a mutable reference to the Rust wrapper from a reference to the native type.
    pub(crate) fn from_native_ref_mut(n: &mut N) -> &mut Self {
        unsafe { transmute_ref_mut(n) }
    }

    /// Converts a pointer to a native value into a pointer to the Rust value.
    pub(crate) fn from_native_ptr(np: *const N) -> *const Self {
        np as _
    }

    /// Converts a pointer to a mutable native value into a pointer to the mutable Rust value.
    #[allow(unused)]
    pub(crate) fn from_native_ptr_mut(np: *mut N) -> *mut Self {
        np as _
    }

    /// Constructs a C++ object in place by calling a
    /// function that expects a pointer that points to
    /// uninitialized memory of the native type.
    pub(crate) fn construct(construct: impl FnOnce(*mut N)) -> Self {
        Self::try_construct(|i| {
            construct(i);
            true
        })
        .unwrap()
    }

    pub(crate) fn try_construct(construct: impl FnOnce(*mut N) -> bool) -> Option<Self> {
        self::try_construct(construct).map(Self::from_native_c)
    }

    /// Replaces the native instance with the one from this Handle, and returns the replaced one
    /// wrapped in a Rust Handle without dropping either one.
    pub(crate) fn replace_native(mut self, native: &mut N) -> Self {
        mem::swap(&mut self.0, native);
        self
    }

    /// Consumes the wrapper and returns the native type.
    pub(crate) fn into_native(mut self) -> N {
        let r = mem::replace(&mut self.0, unsafe { mem::zeroed() });
        mem::forget(self);
        r
    }
}

pub(crate) trait ReplaceWith<Other> {
    fn replace_with(&mut self, other: Other) -> Other;
}

impl<N: NativeDrop> ReplaceWith<Handle<N>> for N {
    fn replace_with(&mut self, other: Handle<N>) -> Handle<N> {
        other.replace_native(self)
    }
}

/// Constructs a C++ object in place by calling a lambda that is meant to initialize
/// the pointer to the Rust memory provided as a pointer.
pub(crate) fn construct<N>(construct: impl FnOnce(*mut N)) -> N {
    try_construct(|i| {
        construct(i);
        true
    })
    .unwrap()
}

pub(crate) fn try_construct<N>(construct: impl FnOnce(*mut N) -> bool) -> Option<N> {
    let mut instance = MaybeUninit::uninit();
    construct(instance.as_mut_ptr()).if_true_then_some(|| unsafe { instance.assume_init() })
}

impl<N: NativeDrop> Drop for Handle<N> {
    fn drop(&mut self) {
        self.0.drop()
    }
}

impl<N: NativeDrop> NativeAccess<N> for Handle<N> {
    fn native(&self) -> &N {
        &self.0
    }

    fn native_mut(&mut self) -> &mut N {
        &mut self.0
    }
}

impl<N: NativeDrop + NativeClone> Clone for Handle<N> {
    fn clone(&self) -> Self {
        Self::from_native_c(self.0.clone())
    }
}

impl<N: NativeDrop + NativePartialEq> PartialEq for Handle<N> {
    fn eq(&self, rhs: &Self) -> bool {
        self.native().eq(rhs.native())
    }
}

impl<N: NativeDrop + NativeHash> Hash for Handle<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.native().hash(state);
    }
}

pub(crate) trait NativeSliceAccess<N: NativeDrop> {
    fn native(&self) -> &[N];
    fn native_mut(&mut self) -> &mut [N];
}

impl<N: NativeDrop> NativeSliceAccess<N> for [Handle<N>] {
    fn native(&self) -> &[N] {
        let ptr = self
            .first()
            .map(|f| f.native() as *const N)
            .unwrap_or(ptr::null());
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    fn native_mut(&mut self) -> &mut [N] {
        let ptr = self
            .first_mut()
            .map(|f| f.native_mut() as *mut N)
            .unwrap_or(ptr::null_mut());
        unsafe { slice::from_raw_parts_mut(ptr, self.len()) }
    }
}

/// A trait that supports retrieving a pointer from an Option<Handle<Native>>.
/// Returns a null pointer if the Option is None.
pub(crate) trait NativePointerOrNull<N> {
    fn native_ptr_or_null(&self) -> *const N;
    unsafe fn native_ptr_or_null_mut_force(&self) -> *mut N;
}

pub(crate) trait NativePointerOrNullMut<N> {
    fn native_ptr_or_null_mut(&mut self) -> *mut N;
}

impl<H, N> NativePointerOrNull<N> for Option<&H>
where
    H: NativeAccess<N>,
{
    fn native_ptr_or_null(&self) -> *const N {
        match self {
            Some(handle) => handle.native(),
            None => ptr::null(),
        }
    }

    unsafe fn native_ptr_or_null_mut_force(&self) -> *mut N {
        match self {
            Some(handle) => handle.native_mut_force(),
            None => ptr::null_mut(),
        }
    }
}

impl<H, N> NativePointerOrNullMut<N> for Option<&mut H>
where
    H: NativeAccess<N>,
{
    fn native_ptr_or_null_mut(&mut self) -> *mut N {
        match self {
            Some(handle) => handle.native_mut(),
            None => ptr::null_mut(),
        }
    }
}

pub(crate) trait NativePointerOrNullMut2<N> {
    fn native_ptr_or_null_mut(&mut self) -> *mut N;
}

pub(crate) trait NativePointerOrNull2<N> {
    fn native_ptr_or_null(&self) -> *const N;
}

impl<H, N> NativePointerOrNull2<N> for Option<&H>
where
    H: NativeTransmutable<N>,
{
    fn native_ptr_or_null(&self) -> *const N {
        match self {
            Some(handle) => handle.native(),
            None => ptr::null(),
        }
    }
}

impl<H, N> NativePointerOrNullMut2<N> for Option<&mut H>
where
    H: NativeTransmutable<N>,
{
    fn native_ptr_or_null_mut(&mut self) -> *mut N {
        match self {
            Some(handle) => handle.native_mut(),
            None => ptr::null_mut(),
        }
    }
}

/// A wrapper type that represents a native type with a pointer to
/// the native object.
#[repr(transparent)]
pub struct RefHandle<N: NativeDrop>(ptr::NonNull<N>);

impl<N: NativeDrop> Drop for RefHandle<N> {
    fn drop(&mut self) {
        self.native_mut().drop()
    }
}

impl<N: NativeDrop> NativeAccess<N> for RefHandle<N> {
    fn native(&self) -> &N {
        unsafe { self.0.as_ref() }
    }
    fn native_mut(&mut self) -> &mut N {
        unsafe { self.0.as_mut() }
    }
}

impl<N: NativeDrop> RefHandle<N> {
    /// Creates a RefHandle from a native pointer.
    ///
    /// From this time on, the handle owns the object that the pointer points
    /// to and will call its NativeDrop implementation if it goes out of scope.
    pub(crate) fn from_ptr(ptr: *mut N) -> Option<Self> {
        ptr::NonNull::new(ptr).map(Self)
    }

    pub(crate) fn into_ptr(self) -> *mut N {
        let p = self.0.as_ptr();
        mem::forget(self);
        p
    }
}

/// A wrapper type represented by a reference counted pointer
/// to the native type.
#[repr(transparent)]
pub struct RCHandle<Native: NativeRefCounted>(ptr::NonNull<Native>);

/// A reference counted handle is cheap to clone, so we do support a conversion
/// from a reference to a ref counter to an owned handle.
impl<N: NativeRefCounted> From<&RCHandle<N>> for RCHandle<N> {
    fn from(rch: &RCHandle<N>) -> Self {
        rch.clone()
    }
}

impl<N: NativeRefCounted> AsRef<RCHandle<N>> for RCHandle<N> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<N: NativeRefCounted> RCHandle<N> {
    /// Creates an reference counted handle from a native pointer.
    ///
    /// Takes ownership of the object the pointer points to, does not increase the reference count.
    ///
    /// Returns `None` if the pointer is `null`.
    #[inline]
    pub(crate) fn from_ptr(ptr: *mut N) -> Option<Self> {
        ptr::NonNull::new(ptr).map(Self)
    }

    /// Creates an reference counted handle from a pointer.
    ///
    /// Returns `None` if the pointer is `null`.
    ///
    /// Shares ownership with the object referenced to by the pointer, therefore increases the
    /// reference count.
    #[inline]
    pub(crate) fn from_unshared_ptr(ptr: *mut N) -> Option<Self> {
        ptr::NonNull::new(ptr).map(|ptr| {
            unsafe { ptr.as_ref()._ref() };
            Self(ptr)
        })
    }

    /// Create a reference to the Rust wrapper from a reference to a pointer that points
    /// to the native type.
    pub(crate) fn from_unshared_ptr_ref(n: &*mut N) -> &Option<Self> {
        unsafe { transmute_ref(n) }
    }
}

#[cfg(tests)]
mod rc_handle_tests {
    use crate::prelude::RCHandle;
    use crate::Typeface;
    use skia_bindings::SkTypeface;
    use std::ptr;

    #[test]
    fn rc_native_ref_null() {
        let f: *mut SkTypeface = ptr::null_mut();
        let r = Typeface::from_native_ref(&f);
        assert!(r.is_none())
    }

    #[test]
    fn rc_native_ref_non_null() {
        let tf = Typeface::default();
        let f: *mut SkTypeface = tf.0;
        let r = Typeface::from_native_ref(&f);
        assert!(r.is_some())
    }
}

impl<N: NativeRefCounted> NativeAccess<N> for RCHandle<N> {
    /// Returns a reference to the native representation.
    fn native(&self) -> &N {
        unsafe { self.0.as_ref() }
    }

    /// Returns a mutable reference to the native representation.
    fn native_mut(&mut self) -> &mut N {
        unsafe { self.0.as_mut() }
    }
}

impl<N: NativeRefCounted> Clone for RCHandle<N> {
    fn clone(&self) -> Self {
        // Support shared mutability when a ref-counted handle is cloned.
        let ptr = self.0;
        unsafe { ptr.as_ref()._ref() };
        Self(ptr)
    }
}

impl<N: NativeRefCounted> Drop for RCHandle<N> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.0.as_ref()._unref() };
    }
}

impl<N: NativeRefCounted + NativePartialEq> PartialEq for RCHandle<N> {
    fn eq(&self, rhs: &Self) -> bool {
        self.native().eq(rhs.native())
    }
}

/// A trait that consumes self and converts it to a ptr to the native type.
pub(crate) trait IntoPtr<N> {
    fn into_ptr(self) -> *mut N;
}

impl<N: NativeRefCounted> IntoPtr<N> for RCHandle<N> {
    fn into_ptr(self) -> *mut N {
        let ptr = self.0.as_ptr();
        mem::forget(self);
        ptr
    }
}

/// A trait that consumes self and converts it to a ptr to the native type or null.
pub(crate) trait IntoPtrOrNull<N> {
    fn into_ptr_or_null(self) -> *mut N;
}

impl<N: NativeRefCounted> IntoPtrOrNull<N> for Option<RCHandle<N>> {
    fn into_ptr_or_null(self) -> *mut N {
        self.map(|rc| rc.into_ptr()).unwrap_or(ptr::null_mut())
    }
}

/// Tag the type to automatically implement get() functions for
/// all Index implementations.
pub trait IndexGet {}

/// Tag the type to automatically implement get() and set() functions
/// for all Index & IndexMut implementation for that type.
pub trait IndexSet {}

pub trait IndexGetter<I, O: Copy> {
    fn get(&self, index: I) -> O;
}

impl<T, I, O: Copy> IndexGetter<I, O> for T
where
    T: Index<I, Output = O> + IndexGet,
{
    fn get(&self, index: I) -> O {
        self[index]
    }
}

pub trait IndexSetter<I, O: Copy> {
    fn set(&mut self, index: I, value: O) -> &mut Self;
}

impl<T, I, O: Copy> IndexSetter<I, O> for T
where
    T: IndexMut<I, Output = O> + IndexSet,
{
    fn set(&mut self, index: I, value: O) -> &mut Self {
        self[index] = value;
        self
    }
}

/// Trait to use native types that as a rust type
/// _inplace_ with the same size and field layout.
pub trait NativeTransmutable<NT: Sized>: Sized
where
    Self: Sized,
{
    /// Provides access to the native value through a
    /// transmuted reference to the Rust value.
    fn native(&self) -> &NT {
        unsafe { transmute_ref(self) }
    }

    /// Provides mutable access to the native value through a
    /// transmuted reference to the Rust value.
    fn native_mut(&mut self) -> &mut NT {
        unsafe { transmute_ref_mut(self) }
    }

    /// Copies the native value to an equivalent Rust value.
    ///
    /// The `_c` suffix is to remind callers that functions that return a native value from a C++
    /// ABI can't be used. For example, C++ member functions must be wrapped in a extern "C" function.
    fn from_native_c(nt: NT) -> Self {
        let r = unsafe { mem::transmute_copy::<NT, Self>(&nt) };
        // don't drop, the Rust type takes over.
        mem::forget(nt);
        r
    }

    /// Copies the rust type to an equivalent instance of the native type.
    fn into_native(self) -> NT {
        let r = unsafe { mem::transmute_copy::<Self, NT>(&self) };
        // don't drop, the native type takes over.
        mem::forget(self);
        r
    }

    /// Provides access to the Rust value through a
    /// transmuted reference to the native value.
    fn from_native_ref(nt: &NT) -> &Self {
        unsafe { transmute_ref(nt) }
    }

    /// Provides access to the Rust value through a
    /// transmuted reference to the native mutable value.
    fn from_native_ref_mut(nt: &mut NT) -> &mut Self {
        unsafe { transmute_ref_mut(nt) }
    }

    /// Converts a pointer to a native value into a pointer to the Rust value.
    fn from_native_ptr(np: *const NT) -> *const Self {
        np as _
    }

    /// Converts a pointer to a mutable native value into a pointer to the mutable Rust value.
    fn from_native_ptr_mut(np: *mut NT) -> *mut Self {
        np as _
    }

    /// Runs a test that proves that the native and the rust
    /// type are of the same size.
    fn test_layout() {
        assert_eq!(mem::size_of::<Self>(), mem::size_of::<NT>());
    }

    fn construct(construct: impl FnOnce(*mut NT)) -> Self {
        Self::try_construct(|i| {
            construct(i);
            true
        })
        .unwrap()
    }

    fn try_construct(construct: impl FnOnce(*mut NT) -> bool) -> Option<Self> {
        self::try_construct(construct).map(Self::from_native_c)
    }
}

pub(crate) trait NativeTransmutableSliceAccess<NT: Sized> {
    fn native(&self) -> &[NT];
    fn native_mut(&mut self) -> &mut [NT];
}

impl<NT, ElementT> NativeTransmutableSliceAccess<NT> for [ElementT]
where
    ElementT: NativeTransmutable<NT>,
{
    fn native(&self) -> &[NT] {
        unsafe { &*(self as *const [ElementT] as *const [NT]) }
    }

    fn native_mut(&mut self) -> &mut [NT] {
        unsafe { &mut *(self as *mut [ElementT] as *mut [NT]) }
    }
}

impl<NT, RustT> NativeTransmutable<Option<NT>> for Option<RustT> where RustT: NativeTransmutable<NT> {}

impl<NT, RustT> NativeTransmutable<Option<&[NT]>> for Option<&[RustT]> where
    RustT: NativeTransmutable<NT>
{
}

pub(crate) trait NativeTransmutableOptionSliceAccessMut<NT: Sized> {
    fn native_mut(&mut self) -> &mut Option<&mut [NT]>;
}

impl<NT, RustT> NativeTransmutableOptionSliceAccessMut<NT> for Option<&mut [RustT]>
where
    RustT: NativeTransmutable<NT>,
{
    fn native_mut(&mut self) -> &mut Option<&mut [NT]> {
        unsafe { transmute_ref_mut(self) }
    }
}

//
// Convenience functions to access Option<&[]> as optional ptr (opt_ptr)
// that may be null.
//

pub(crate) trait AsPointerOrNull<PointerT> {
    fn as_ptr_or_null(&self) -> *const PointerT;
}

pub(crate) trait AsPointerOrNullMut<PointerT>: AsPointerOrNull<PointerT> {
    fn as_ptr_or_null_mut(&mut self) -> *mut PointerT;
}

impl<E> AsPointerOrNull<E> for Option<E> {
    fn as_ptr_or_null(&self) -> *const E {
        match self {
            Some(e) => e,
            None => ptr::null(),
        }
    }
}

impl<E> AsPointerOrNullMut<E> for Option<E> {
    fn as_ptr_or_null_mut(&mut self) -> *mut E {
        match self {
            Some(e) => e,
            None => ptr::null_mut(),
        }
    }
}

impl<E> AsPointerOrNull<E> for Option<&[E]> {
    fn as_ptr_or_null(&self) -> *const E {
        match self {
            Some(slice) => slice.as_ptr(),
            None => ptr::null(),
        }
    }
}

impl<E> AsPointerOrNull<E> for Option<&mut [E]> {
    fn as_ptr_or_null(&self) -> *const E {
        match self {
            Some(slice) => slice.as_ptr(),
            None => ptr::null(),
        }
    }
}

impl<E> AsPointerOrNullMut<E> for Option<&mut [E]> {
    fn as_ptr_or_null_mut(&mut self) -> *mut E {
        match self {
            Some(slice) => slice.as_mut_ptr(),
            None => ptr::null_mut(),
        }
    }
}

impl<E> AsPointerOrNull<E> for Option<&Vec<E>> {
    fn as_ptr_or_null(&self) -> *const E {
        match self {
            Some(v) => v.as_ptr(),
            None => ptr::null(),
        }
    }
}

impl<E> AsPointerOrNull<E> for Option<Vec<E>> {
    fn as_ptr_or_null(&self) -> *const E {
        match self {
            Some(v) => v.as_ptr(),
            None => ptr::null(),
        }
    }
}

impl<E> AsPointerOrNullMut<E> for Option<Vec<E>> {
    fn as_ptr_or_null_mut(&mut self) -> *mut E {
        match self {
            Some(v) => v.as_mut_ptr(),
            None => ptr::null_mut(),
        }
    }
}

// Wraps a handle so that the Rust's borrow checker assumes it represents
// something that borrows something else.
#[repr(transparent)]
pub struct Borrows<'a, H>(H, PhantomData<&'a ()>);

impl<'a, H> Deref for Borrows<'a, H> {
    type Target = H;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO: this is most likely unsafe because someone could replace the
// value the reference is pointing to.
impl<'a, H> DerefMut for Borrows<'a, H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, H> Borrows<'a, H> {
    /// Notify that the borrowed dependency is not referred to anymore and return the handle.
    /// # Safety
    /// The borrowed dependency must be removed before calling `release()`.
    pub unsafe fn release(self) -> H {
        self.0
    }
}

pub(crate) trait BorrowsFrom: Sized {
    fn borrows<D: ?Sized>(self, _dep: &D) -> Borrows<Self>;
}

impl<T: Sized> BorrowsFrom for T {
    fn borrows<D: ?Sized>(self, _dep: &D) -> Borrows<Self> {
        Borrows(self, PhantomData)
    }
}

/// Declares a base class for a native type.
pub trait NativeBase<Base> {
    fn base(&self) -> &Base {
        unsafe { &*(self as *const Self as *const Base) }
    }

    fn base_mut(&mut self) -> &mut Base {
        unsafe { &mut *(self as *mut Self as *mut Base) }
    }
}

pub struct Sendable<H: ConditionallySend>(H);
unsafe impl<H: ConditionallySend> Send for Sendable<H> {}

impl<H: ConditionallySend> Sendable<H> {
    pub fn unwrap(self) -> H {
        self.0
    }
}

pub trait ConditionallySend: Sized {
    /// Returns `true` if the handle can be sent to another thread.
    fn can_send(&self) -> bool;
    /// Wrap the handle in a type that can be sent to another thread and unwrapped there.
    ///
    /// Guaranteed to succeed of can_send() returns `true`.
    fn wrap_send(self) -> Result<Sendable<Self>, Self>;
}

/// `RCHandle<H>` is conditionally Send and can be sent to
/// another thread when its reference count is 1.
impl<H: NativeRefCountedBase> ConditionallySend for RCHandle<H> {
    fn can_send(&self) -> bool {
        self.native().unique()
    }

    fn wrap_send(self) -> Result<Sendable<Self>, Self> {
        if self.can_send() {
            Ok(Sendable(self))
        } else {
            Err(self)
        }
    }
}

/// Functions that are (supposedly) _safer_ variants of the ones Rust provides.
pub(crate) mod safer {
    use core::slice;
    use std::ptr;

    /// Invokes [slice::from_raw_parts] with the `ptr` only when `len` != 0, otherwise passes
    /// `ptr::NonNull::dangling()` as recommended.
    ///
    /// Panics if `len` != 0 and `ptr` is `null`.
    pub unsafe fn from_raw_parts<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
        let ptr = if len == 0 {
            ptr::NonNull::dangling().as_ptr()
        } else {
            assert!(!ptr.is_null());
            ptr
        };
        slice::from_raw_parts(ptr, len)
    }

    /// Invokes [slice::from_raw_parts_mut] with the `ptr` only if `len` != 0, otherwise passes
    /// `ptr::NonNull::dangling()` as recommended.
    ///
    /// Panics if `len` != 0 and `ptr` is `null`.
    pub unsafe fn from_raw_parts_mut<'a, T>(ptr: *mut T, len: usize) -> &'a mut [T] {
        let ptr = if len == 0 {
            ptr::NonNull::dangling().as_ptr() as *mut _
        } else {
            assert!(!ptr.is_null());
            ptr
        };
        slice::from_raw_parts_mut(ptr, len)
    }
}
