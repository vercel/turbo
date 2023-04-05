use std::{
    borrow::{Borrow, Cow},
    cmp::Ordering,
    collections::TryReserveError,
    error,
    ffi::{OsStr, OsString},
    fmt,
    fs::{self, Metadata, ReadDir},
    hash::Hash,
    io::{self, Result},
    iter::{self, FusedIterator},
    ops::{Deref, DerefMut},
    path::{Components, Display, Iter, Path, PathBuf, StripPrefixError},
    rc::Rc,
    result::Result as StdResult,
    str::FromStr,
    sync::Arc,
};

use delegate::delegate;

use crate::AnchoredSystemPath;

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct AbsoluteSystemPathBuf(PathBuf);

impl AbsoluteSystemPathBuf {
    #[must_use]
    pub fn as_absolute_system_path(&self) -> &AbsoluteSystemPath {
        unsafe { AbsoluteSystemPath::coerce_absolute_system_path(&self.0) }
    }

    #[must_use]
    pub fn into_boxed_absolute_system_path(self) -> Box<AbsoluteSystemPath> {
        let ptr = Box::into_raw(self.0.into_boxed_path()) as *mut AbsoluteSystemPath;
        unsafe { Box::from_raw(ptr) }
    }

    #[must_use]
    pub fn from_path_buf(path: PathBuf) -> StdResult<AbsoluteSystemPathBuf, FromError> {
        if path.is_absolute() {
            Ok(AbsoluteSystemPathBuf(path))
        } else {
            Err(FromError(()))
        }
    }

    // API OVERRIDES

    pub fn push<P: AsRef<AnchoredSystemPath>>(&mut self, path: P) {
        self.0.push(path.as_ref())
    }

    // This is a static method, it can't be delegated.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf(PathBuf::with_capacity(capacity))
    }

    delegate! {
        to self.0 {
            pub fn as_path(&self) -> &Path;
            pub fn capacity(&self) -> usize;
            // INVALID pub fn clear(&mut self);
            pub fn into_boxed_path(self) -> Box<Path>;
            pub fn into_os_string(self) -> OsString;
            pub fn pop(&mut self) -> bool;
            // OVERRIDE pub fn push<P: AsRef<Path>>(&mut self, path: P);
            pub fn reserve(&mut self, additional: usize);
            pub fn reserve_exact(&mut self, additional: usize);
            pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool;
            pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S);
            pub fn shrink_to(&mut self, min_capacity: usize);
            pub fn shrink_to_fit(&mut self);
            pub fn try_reserve(&mut self, additional: usize) -> StdResult<(), TryReserveError>;
            pub fn try_reserve_exact(&mut self, additional: usize) -> StdResult<(), TryReserveError>;
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct AbsoluteSystemPath(Path);

impl AbsoluteSystemPath {
    pub fn new(s: &(impl AsRef<OsStr> + ?Sized)) -> StdResult<&AbsoluteSystemPath, FromError> {
        let path = Path::new(s.as_ref());
        if path.is_absolute() {
            Ok(unsafe { AbsoluteSystemPath::coerce_absolute_system_path(path) })
        } else {
            Err(FromError(()))
        }
    }

    // MANUAL IMPLEMENTATIONS

    #[must_use]
    pub fn into_absolute_system_path_buf(self: Box<AbsoluteSystemPath>) -> AbsoluteSystemPathBuf {
        let ptr = Box::into_raw(self) as *mut Path;
        let boxed_path = unsafe { Box::from_raw(ptr) };
        AbsoluteSystemPathBuf(boxed_path.into_path_buf())
    }

    #[must_use]
    pub fn to_absolute_system_path_buf(&self) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf(self.0.to_path_buf())
    }

    #[must_use]
    pub fn from_path(path: &Path) -> StdResult<&AbsoluteSystemPath, FromError> {
        AbsoluteSystemPath::new(path.as_os_str())
    }

    #[must_use]
    unsafe fn coerce_absolute_system_path(path: &Path) -> &AbsoluteSystemPath {
        &*(path as *const Path as *const AbsoluteSystemPath)
    }

    unsafe fn coerce_absolute_system_path_mut(path: &mut Path) -> &mut AbsoluteSystemPath {
        &mut *(path as *mut Path as *mut AbsoluteSystemPath)
    }

    // API OVERRIDES
    // These explicitly change the method signature.

    #[inline]
    pub fn ancestors(&self) -> AbsoluteSystemPathAncestors<'_> {
        AbsoluteSystemPathAncestors { next: Some(&self) }
    }

    pub fn canonicalize(&self) -> Result<AbsoluteSystemPathBuf> {
        fs::canonicalize(self).and_then(|path| path.try_into().map_err(FromError::into_io_error))
    }

    #[must_use]
    pub fn into_path_buf(self: Box<AbsoluteSystemPath>) -> PathBuf {
        let ptr = Box::into_raw(self) as *mut Path;
        let boxed_path = unsafe { Box::from_raw(ptr) };
        boxed_path.into_path_buf()
    }

    #[must_use]
    pub fn join<P: AsRef<AnchoredSystemPath>>(&self, path: P) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf(self.0.join(&path.as_ref()))
    }

    #[must_use]
    pub fn parent(&self) -> Option<&AbsoluteSystemPath> {
        self.0
            .parent()
            .and_then(|path| Some(unsafe { AbsoluteSystemPath::coerce_absolute_system_path(path) }))
    }

    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf(self.0.with_extension(extension.as_ref()))
    }

    #[must_use]
    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf(self.0.with_file_name(file_name.as_ref()))
    }

    delegate! {
        to self.0 {
            // OVERRIDE pub fn ancestors(&self) -> Ancestors<'_>;
            pub fn as_os_str(&self) -> &OsStr;
            // OVERRIDE pub fn canonicalize(&self) -> Result<PathBuf>;
            pub fn components(&self) -> Components<'_>;
            pub fn display(&self) -> Display<'_>;
            pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool;
            pub fn exists(&self) -> bool;
            pub fn extension(&self) -> Option<&OsStr>;
            pub fn file_name(&self) -> Option<&OsStr>;
            pub fn file_stem(&self) -> Option<&OsStr>;
            pub fn has_root(&self) -> bool;
            // OVERRIDE pub fn into_path_buf(self: Box<Path>) -> PathBuf;
            pub fn is_absolute(&self) -> bool;
            pub fn is_dir(&self) -> bool;
            pub fn is_file(&self) -> bool;
            pub fn is_relative(&self) -> bool;
            pub fn is_symlink(&self) -> bool;
            pub fn iter(&self) -> Iter<'_>;
            // OVERRIDE pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf;
            pub fn metadata(&self) -> Result<Metadata>;
            // OVERRIDE pub fn parent(&self) -> Option<&Path>;
            pub fn read_dir(&self) -> Result<ReadDir>;
            pub fn read_link(&self) -> Result<PathBuf>;
            pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool;
            pub fn strip_prefix<P>(&self, base: P) -> StdResult<&Path, StripPrefixError>
                where P: AsRef<Path>;
            pub fn symlink_metadata(&self) -> Result<Metadata>;
            pub fn to_path_buf(&self) -> PathBuf;
            pub fn to_str(&self) -> Option<&str>;
            pub fn to_string_lossy(&self) -> Cow<'_, str>;
            pub fn try_exists(&self) -> Result<bool>;
            // OVERRIDE pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;
            // OVERRIDE pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> PathBuf;
        }
    }
}

// Ancestors
// All ancestors of an AbsoluteSystemPath are _also_ AbsoluteSystemPaths.

#[derive(Copy, Clone, Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct AbsoluteSystemPathAncestors<'a> {
    next: Option<&'a AbsoluteSystemPath>,
}

impl<'a> Iterator for AbsoluteSystemPathAncestors<'a> {
    type Item = &'a AbsoluteSystemPath;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.next;
        self.next = next.and_then(AbsoluteSystemPath::parent);
        next
    }
}

impl FusedIterator for AbsoluteSystemPathAncestors<'_> {}

// Direct Iteration

impl<'a> IntoIterator for &'a AbsoluteSystemPathBuf {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a AbsoluteSystemPath {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<P: AsRef<AnchoredSystemPath>> iter::Extend<P> for AbsoluteSystemPathBuf {
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |p| self.push(p.as_ref()));
    }
}

// AsRef
// Only the things which absolutely cannot fail.

impl AsRef<AbsoluteSystemPath> for AbsoluteSystemPath {
    #[inline]
    fn as_ref(&self) -> &AbsoluteSystemPath {
        self
    }
}

impl AsRef<AbsoluteSystemPath> for AbsoluteSystemPathBuf {
    #[inline]
    fn as_ref(&self) -> &AbsoluteSystemPath {
        self.as_absolute_system_path()
    }
}

impl AsRef<OsStr> for AbsoluteSystemPath {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl AsRef<OsStr> for AbsoluteSystemPathBuf {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl AsRef<Path> for AbsoluteSystemPath {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for AbsoluteSystemPathBuf {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

// Borrowing

impl Deref for AbsoluteSystemPathBuf {
    type Target = AbsoluteSystemPath;

    #[inline]
    fn deref(&self) -> &AbsoluteSystemPath {
        self.as_absolute_system_path()
    }
}

impl DerefMut for AbsoluteSystemPathBuf {
    #[inline]
    fn deref_mut(&mut self) -> &mut AbsoluteSystemPath {
        unsafe { AbsoluteSystemPath::coerce_absolute_system_path_mut(&mut self.0) }
    }
}

impl Borrow<AbsoluteSystemPath> for AbsoluteSystemPathBuf {
    #[inline]
    fn borrow(&self) -> &AbsoluteSystemPath {
        self.deref()
    }
}

impl ToOwned for AbsoluteSystemPath {
    type Owned = AbsoluteSystemPathBuf;

    #[inline]
    fn to_owned(&self) -> AbsoluteSystemPathBuf {
        self.to_absolute_system_path_buf()
    }
}

// Clone

impl Clone for Box<AbsoluteSystemPath> {
    #[inline]
    fn clone(&self) -> Self {
        self.to_absolute_system_path_buf()
            .into_boxed_absolute_system_path()
    }
}

// From<AbsoluteSystemPath(Buf)> for T

impl<T: ?Sized + AsRef<AbsoluteSystemPath>> From<&T> for AbsoluteSystemPathBuf {
    fn from(s: &T) -> AbsoluteSystemPathBuf {
        AbsoluteSystemPathBuf::from(s.as_ref().to_owned())
    }
}

impl From<AbsoluteSystemPathBuf> for PathBuf {
    fn from(path: AbsoluteSystemPathBuf) -> PathBuf {
        path.0
    }
}

impl From<&AbsoluteSystemPath> for Arc<Path> {
    /// Converts a [`AbsoluteSystemPath`] into an [`Arc`] by copying the
    /// [`AbsoluteSystemPath`] data into a new [`Arc`] buffer.
    #[inline]
    fn from(s: &AbsoluteSystemPath) -> Arc<Path> {
        let arc: Arc<OsStr> = Arc::from(s.as_os_str());
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const Path) }
    }
}

impl From<&AbsoluteSystemPath> for Arc<AbsoluteSystemPath> {
    /// Converts a [`AbsoluteSystemPath`] into an [`Arc`] by copying the
    /// [`AbsoluteSystemPath`] data into a new [`Arc`] buffer.
    #[inline]
    fn from(s: &AbsoluteSystemPath) -> Arc<AbsoluteSystemPath> {
        let arc: Arc<OsStr> = Arc::from(s.as_os_str());
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const AbsoluteSystemPath) }
    }
}

impl From<&AbsoluteSystemPath> for Box<Path> {
    /// Creates a boxed [`Path`] from a reference.
    ///
    /// This will allocate and clone `path` to it.
    fn from(path: &AbsoluteSystemPath) -> Box<Path> {
        let boxed: Box<OsStr> = path.as_os_str().into();
        let rw = Box::into_raw(boxed) as *mut Path;
        unsafe { Box::from_raw(rw) }
    }
}

impl From<&AbsoluteSystemPath> for Box<AbsoluteSystemPath> {
    /// Creates a boxed [`AbsoluteSystemPath`] from a reference.
    ///
    /// This will allocate and clone `path` to it.
    fn from(path: &AbsoluteSystemPath) -> Box<AbsoluteSystemPath> {
        let boxed: Box<OsStr> = path.as_os_str().into();
        let rw = Box::into_raw(boxed) as *mut AbsoluteSystemPath;
        unsafe { Box::from_raw(rw) }
    }
}

impl From<&AbsoluteSystemPath> for Rc<Path> {
    /// Converts a [`AbsoluteSystemPath`] into an [`Rc`] by copying the
    /// [`AbsoluteSystemPath`] data into a new [`Rc`] buffer.
    #[inline]
    fn from(s: &AbsoluteSystemPath) -> Rc<Path> {
        let rc: Rc<OsStr> = Rc::from(s.as_os_str());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const Path) }
    }
}

impl From<&AbsoluteSystemPath> for Rc<AbsoluteSystemPath> {
    /// Converts a [`AbsoluteSystemPath`] into an [`Rc`] by copying the
    /// [`AbsoluteSystemPath`] data into a new [`Rc`] buffer.
    #[inline]
    fn from(s: &AbsoluteSystemPath) -> Rc<AbsoluteSystemPath> {
        let rc: Rc<OsStr> = Rc::from(s.as_os_str());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const AbsoluteSystemPath) }
    }
}

impl From<Box<AbsoluteSystemPath>> for PathBuf {
    /// Converts a <code>[Box]&lt;[AbsoluteSystemPath]&gt;</code> into a
    /// [`PathBuf`].
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(boxed: Box<AbsoluteSystemPath>) -> PathBuf {
        boxed.into_path_buf()
    }
}

impl From<Box<AbsoluteSystemPath>> for AbsoluteSystemPathBuf {
    /// Converts a <code>[Box]&lt;[AbsoluteSystemPath]&gt;</code> into a
    /// [`AbsoluteSystemPathBuf`].
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(boxed: Box<AbsoluteSystemPath>) -> AbsoluteSystemPathBuf {
        boxed.into_absolute_system_path_buf()
    }
}

impl From<AbsoluteSystemPathBuf> for Arc<Path> {
    /// Converts a [`AbsoluteSystemPathBuf`] into an <code>[Arc]<[Path]></code>
    /// by moving the [`AbsoluteSystemPathBuf`] data into a new [`Arc`]
    /// buffer.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Arc<Path> {
        let arc: Arc<OsStr> = Arc::from(s.into_os_string());
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const Path) }
    }
}

impl From<AbsoluteSystemPathBuf> for Arc<AbsoluteSystemPath> {
    /// Converts a [`AbsoluteSystemPathBuf`] into an
    /// <code>[Arc]<[AbsoluteSystemPath]></code> by moving the
    /// [`AbsoluteSystemPathBuf`] data into a new [`Arc`] buffer.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Arc<AbsoluteSystemPath> {
        let arc: Arc<OsStr> = Arc::from(s.into_os_string());
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const AbsoluteSystemPath) }
    }
}

impl From<AbsoluteSystemPathBuf> for Box<Path> {
    /// Converts a [`AbsoluteSystemPathBuf`] into a
    /// <code>[Box]&lt;[Path]&gt;</code>.
    ///
    /// This conversion currently should not allocate memory,
    /// but this behavior is not guaranteed on all platforms or in all future
    /// versions.
    #[inline]
    fn from(p: AbsoluteSystemPathBuf) -> Box<Path> {
        p.into_boxed_path()
    }
}

impl From<AbsoluteSystemPathBuf> for Box<AbsoluteSystemPath> {
    /// Converts a [`AbsoluteSystemPathBuf`] into a
    /// <code>[Box]&lt;[AbsoluteSystemPath]&gt;</code>.
    ///
    /// This conversion currently should not allocate memory,
    /// but this behavior is not guaranteed on all platforms or in all future
    /// versions.
    #[inline]
    fn from(p: AbsoluteSystemPathBuf) -> Box<AbsoluteSystemPath> {
        p.into_boxed_absolute_system_path()
    }
}

impl From<AbsoluteSystemPathBuf> for OsString {
    /// Converts a [`AbsoluteSystemPathBuf`] into an [`OsString`]
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(path_buf: AbsoluteSystemPathBuf) -> OsString {
        path_buf.0.into_os_string()
    }
}

impl From<AbsoluteSystemPathBuf> for Rc<Path> {
    /// Converts a [`AbsoluteSystemPathBuf`] into an <code>[Rc]<[Path]></code>
    /// by moving the [`AbsoluteSystemPathBuf`] data into a new [`Rc`]
    /// buffer.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Rc<Path> {
        let rc: Rc<OsStr> = Rc::from(s.into_os_string());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const Path) }
    }
}

impl From<AbsoluteSystemPathBuf> for Rc<AbsoluteSystemPath> {
    /// Converts a [`AbsoluteSystemPathBuf`] into an
    /// <code>[Rc]<[AbsoluteSystemPath]></code> by moving the
    /// [`AbsoluteSystemPathBuf`] data into a new [`Rc`] buffer.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Rc<AbsoluteSystemPath> {
        let rc: Rc<OsStr> = Rc::from(s.into_os_string());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const AbsoluteSystemPath) }
    }
}

impl<'a> From<AbsoluteSystemPathBuf> for Cow<'a, Path> {
    /// Creates a clone-on-write pointer from an owned
    /// instance of [`PathBuf`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Cow<'a, Path> {
        Cow::Owned(s.0)
    }
}

impl<'a> From<AbsoluteSystemPathBuf> for Cow<'a, AbsoluteSystemPath> {
    /// Creates a clone-on-write pointer from an owned
    /// instance of [`PathBuf`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(s: AbsoluteSystemPathBuf) -> Cow<'a, AbsoluteSystemPath> {
        Cow::Owned(s)
    }
}

impl<'a> From<Cow<'a, AbsoluteSystemPath>> for AbsoluteSystemPathBuf {
    /// Converts a clone-on-write pointer to an owned path.
    ///
    /// Converting from a `Cow::Owned` does not clone or allocate.
    #[inline]
    fn from(p: Cow<'a, AbsoluteSystemPath>) -> Self {
        p.into_owned()
    }
}

impl<'a> From<&'a AbsoluteSystemPathBuf> for Cow<'a, Path> {
    /// Creates a clone-on-write pointer from a reference to
    /// [`AbsoluteSystemPathBuf`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(p: &'a AbsoluteSystemPathBuf) -> Cow<'a, Path> {
        Cow::Borrowed(p.as_path())
    }
}

impl<'a> From<&'a AbsoluteSystemPathBuf> for Cow<'a, AbsoluteSystemPath> {
    /// Creates a clone-on-write pointer from a reference to
    /// [`AbsoluteSystemPathBuf`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(p: &'a AbsoluteSystemPathBuf) -> Cow<'a, AbsoluteSystemPath> {
        Cow::Borrowed(p.as_absolute_system_path())
    }
}

impl<'a> From<&'a AbsoluteSystemPath> for Cow<'a, AbsoluteSystemPath> {
    /// Creates a clone-on-write pointer from a reference to
    /// [`AbsoluteSystemPath`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(s: &'a AbsoluteSystemPath) -> Cow<'a, AbsoluteSystemPath> {
        Cow::Borrowed(s)
    }
}

impl<'a> From<&'a AbsoluteSystemPath> for Cow<'a, Path> {
    /// Creates a clone-on-write pointer from a reference to
    /// [`AbsoluteSystemPath`].
    ///
    /// This conversion does not clone or allocate.
    #[inline]
    fn from(s: &'a AbsoluteSystemPath) -> Cow<'a, Path> {
        Cow::Borrowed(s.as_ref())
    }
}

impl From<Cow<'_, AbsoluteSystemPath>> for Box<AbsoluteSystemPath> {
    /// Creates a boxed [`AbsoluteSystemPath`] from a clone-on-write pointer.
    ///
    /// Converting from a `Cow::Owned` does not clone or allocate.
    #[inline]
    fn from(cow: Cow<'_, AbsoluteSystemPath>) -> Box<AbsoluteSystemPath> {
        match cow {
            Cow::Borrowed(path) => Box::from(path),
            Cow::Owned(path) => Box::from(path),
        }
    }
}

// TryFrom<T> for AbsoluteSystemPath(Buf)

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FromError(());

impl FromError {
    /// Converts self into a [`std::io::Error`] with kind
    /// [`InvalidData`](io::ErrorKind::InvalidData).
    ///
    /// Many users of `FromError` will want to convert it into an `io::Error`.
    /// This is a convenience method to do that.
    pub fn into_io_error(self) -> io::Error {
        // NOTE: we don't currently implement `From<FromError> for io::Error` because we
        // want to ensure the user actually desires that conversion.
        io::Error::new(io::ErrorKind::InvalidData, self)
    }
}

impl fmt::Display for FromError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Path is not an AbsoluteSystemPath")
    }
}

impl error::Error for FromError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl TryFrom<PathBuf> for AbsoluteSystemPathBuf {
    type Error = FromError;

    fn try_from(path: PathBuf) -> StdResult<AbsoluteSystemPathBuf, Self::Error> {
        AbsoluteSystemPathBuf::from_path_buf(path)
    }
}

impl<'a> TryFrom<&'a Path> for &'a AbsoluteSystemPath {
    type Error = FromError;

    fn try_from(path: &'a Path) -> StdResult<&'a AbsoluteSystemPath, Self::Error> {
        AbsoluteSystemPath::from_path(path)
    }
}

impl TryFrom<OsString> for AbsoluteSystemPathBuf {
    type Error = FromError;

    fn try_from(path: OsString) -> StdResult<AbsoluteSystemPathBuf, Self::Error> {
        AbsoluteSystemPathBuf::from_path_buf(path.into())
    }
}

impl TryFrom<String> for AbsoluteSystemPathBuf {
    type Error = FromError;

    fn try_from(path: String) -> StdResult<AbsoluteSystemPathBuf, Self::Error> {
        AbsoluteSystemPathBuf::from_path_buf(path.into())
    }
}

impl TryFrom<&str> for AbsoluteSystemPathBuf {
    type Error = FromError;

    fn try_from(path: &str) -> StdResult<AbsoluteSystemPathBuf, Self::Error> {
        AbsoluteSystemPathBuf::from_path_buf(path.into())
    }
}

impl FromStr for AbsoluteSystemPathBuf {
    type Err = FromError;

    #[inline]
    fn from_str(path: &str) -> StdResult<AbsoluteSystemPathBuf, Self::Err> {
        AbsoluteSystemPathBuf::from_path_buf(path.into())
    }
}

// Comparison
macro_rules! impl_cmp {
    (<$($life:lifetime),*> $lhs:ty, $rhs: ty) => {
        impl<$($life),*> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <AbsoluteSystemPath as PartialEq>::eq(self, other)
            }
        }

        impl<$($life),*> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <AbsoluteSystemPath as PartialEq>::eq(self, other)
            }
        }

        impl<$($life),*> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <AbsoluteSystemPath as PartialOrd>::partial_cmp(self, other)
            }
        }

        impl<$($life),*> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <AbsoluteSystemPath as PartialOrd>::partial_cmp(self, other)
            }
        }
    };
}

impl_cmp!(<> AbsoluteSystemPathBuf, AbsoluteSystemPath);
impl_cmp!(<'a> AbsoluteSystemPathBuf, &'a AbsoluteSystemPath);
impl_cmp!(<'a> Cow<'a, AbsoluteSystemPath>, AbsoluteSystemPath);
impl_cmp!(<'a, 'b> Cow<'a, AbsoluteSystemPath>, &'b AbsoluteSystemPath);
impl_cmp!(<'a> Cow<'a, AbsoluteSystemPath>, AbsoluteSystemPathBuf);

macro_rules! impl_cmp_std_path {
    (<$($life:lifetime),*> $lhs:ty, $rhs: ty) => {
        impl<$($life),*> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other)
            }
        }

        impl<$($life),*> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self, other.as_ref())
            }
        }

        impl<$($life),*> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other)
            }
        }

        impl<$($life),*> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self, other.as_ref())
            }
        }
    };
}

impl_cmp_std_path!(<> AbsoluteSystemPathBuf, Path);
impl_cmp_std_path!(<'a> AbsoluteSystemPathBuf, &'a Path);
impl_cmp_std_path!(<'a> AbsoluteSystemPathBuf, Cow<'a, Path>);
impl_cmp_std_path!(<> AbsoluteSystemPathBuf, PathBuf);
impl_cmp_std_path!(<> AbsoluteSystemPath, Path);
impl_cmp_std_path!(<'a> AbsoluteSystemPath, &'a Path);
impl_cmp_std_path!(<'a> AbsoluteSystemPath, Cow<'a, Path>);
impl_cmp_std_path!(<> AbsoluteSystemPath, PathBuf);
impl_cmp_std_path!(<'a> &'a AbsoluteSystemPath, Path);
impl_cmp_std_path!(<'a, 'b> &'a AbsoluteSystemPath, Cow<'b, Path>);
impl_cmp_std_path!(<'a> &'a AbsoluteSystemPath, PathBuf);

macro_rules! impl_cmp_os_str {
    (<$($life:lifetime),*> $lhs:ty, $rhs: ty) => {
        impl<$($life),*> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl<$($life),*> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl<$($life),*> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other.as_ref())
            }
        }

        impl<$($life),*> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other.as_ref())
            }
        }
    };
}

impl_cmp_os_str!(<> AbsoluteSystemPathBuf, OsStr);
impl_cmp_os_str!(<'a> AbsoluteSystemPathBuf, &'a OsStr);
impl_cmp_os_str!(<'a> AbsoluteSystemPathBuf, Cow<'a, OsStr>);
impl_cmp_os_str!(<> AbsoluteSystemPathBuf, OsString);
impl_cmp_os_str!(<> AbsoluteSystemPath, OsStr);
impl_cmp_os_str!(<'a> AbsoluteSystemPath, &'a OsStr);
impl_cmp_os_str!(<'a> AbsoluteSystemPath, Cow<'a, OsStr>);
impl_cmp_os_str!(<> AbsoluteSystemPath, OsString);
impl_cmp_os_str!(<'a> &'a AbsoluteSystemPath, OsStr);
impl_cmp_os_str!(<'a, 'b> &'a AbsoluteSystemPath, Cow<'b, OsStr>);
impl_cmp_os_str!(<'a> &'a AbsoluteSystemPath, OsString);
