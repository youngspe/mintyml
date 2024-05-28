use std::{
    borrow::{Borrow, BorrowMut},
    ffi::{OsStr, OsString},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

pub fn default<T: Default>() -> T {
    Default::default()
}

pub trait ResultExt:
    Into<Result<Self::OkVal, Self::ErrVal>> + From<Result<Self::OkVal, Self::ErrVal>>
{
    type OkVal;
    type ErrVal;
    fn into_result(self) -> Result<Self::OkVal, Self::ErrVal> {
        self.into()
    }
}

impl<T, E> ResultExt for Result<T, E> {
    type OkVal = T;
    type ErrVal = E;
}

pub trait OptionExt: Into<Option<Self::Val>> + From<Option<Self::Val>> {
    type Val;
    fn into_option(self) -> Option<Self::Val> {
        self.into()
    }
}

impl<T> OptionExt for Option<T> {
    type Val = T;
}

pub trait UtilExt: Sized {
    fn also(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    fn also_with<T>(mut self, f: impl FnOnce(&mut Self) -> T) -> (Self, T) {
        let out = f(&mut self);
        (self, out)
    }

    fn also_with_ok<T, E>(
        mut self,
        f: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<(Self, T), E> {
        f(&mut self).map(|out| (self, out))
    }

    fn also_with_some<T>(mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<(Self, T)> {
        f(&mut self).map(|out| (self, out))
    }

    fn also_ok<E>(mut self, f: impl FnOnce(&mut Self) -> Result<(), E>) -> Result<Self, E> {
        f(&mut self).map(|()| self)
    }

    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }

    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    fn wrap_some(self) -> Option<Self> {
        Some(self)
    }

    fn pipe_ok<T, E>(self, f: impl FnOnce(Self) -> Result<T, E>) -> Result<T, E> {
        self.pipe(f)
    }

    fn pipe_some<T>(self, f: impl FnOnce(Self) -> Option<T>) -> Option<T> {
        self.pipe(f)
    }

    fn into_<T>(self) -> T
    where
        Self: Into<T>,
    {
        self.into()
    }

    fn try_into_<T>(self) -> Result<T, Self::Error>
    where
        Self: TryInto<T>,
    {
        self.try_into()
    }

    fn drop_ok(self) -> Result<(), Self::ErrVal>
    where
        Self: ResultExt,
    {
        self.into_result().map(|_| ())
    }

    fn err_into<E>(self) -> Result<Self::OkVal, E>
    where
        Self: ResultExt,
        Self::ErrVal: Into<E>,
    {
        self.into_result().map_err(Into::into)
    }

    fn try_map<Out, E>(self, f: impl FnOnce(Self::Val) -> Result<Out, E>) -> Result<Option<Out>, E>
    where
        Self: OptionExt,
    {
        self.into_option().map(f).transpose()
    }

    fn try_and_then<Out, E>(
        self,
        f: impl FnOnce(Self::Val) -> Result<Option<Out>, E>,
    ) -> Result<Option<Out>, E>
    where
        Self: OptionExt,
    {
        self.into_option()
            .map(f)
            .transpose()
            .map(|opt| opt.flatten())
    }
}

impl<T> UtilExt for T {}

macro_rules! try_with_context {
    ($result:expr, $($cx:expr),* $(,)?) => {
        match $result {
            Ok(x) => x,
            Err(e) => {
                use ::anyhow::Context;
                match Err::<
                    ::core::convert::Infallible,
                    _,
                >(e)$(.context($cx))* ? {}
            },
        }
    };
    ($result:expr) => { $result? };
}

pub trait PathExt: AsRef<Path> {
    fn has_trailing_slash(&self) -> bool {
        let this = self.as_ref();
        this.ends_with("/") || this.ends_with(std::path::MAIN_SEPARATOR_STR)
    }

    fn as_path(&self) -> &Path {
        self.as_ref()
    }

    fn into_buf(self) -> PathBuf
    where
        Self: Sized,
    {
        self.as_path().into()
    }

    fn into_arc(self) -> ArcPath
    where
        Self: Sized,
    {
        self.as_path().into()
    }
}

impl PathExt for Path {}
impl PathExt for PathBuf {
    fn into_buf(self) -> PathBuf {
        self
    }
    fn into_arc(self) -> ArcPath {
        self.into()
    }
}

impl PathExt for &Path {}
impl PathExt for &PathBuf {}

impl PathExt for &ArcPath {
    fn into_arc(self) -> ArcPath {
        self.clone()
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArcCow<T>(Arc<T>);

impl<T> Clone for ArcCow<T> {
    fn clone(&self) -> Self {
        self.0.clone().into()
    }
}

impl<T> From<ArcCow<T>> for Arc<T> {
    fn from(value: ArcCow<T>) -> Self {
        value.0
    }
}

impl<T> From<T> for ArcCow<T> {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl<T> From<Arc<T>> for ArcCow<T> {
    fn from(value: Arc<T>) -> Self {
        Self(value)
    }
}

impl<T: Clone> From<&T> for ArcCow<T> {
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl<T> Borrow<T> for ArcCow<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> BorrowMut<T> for ArcCow<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut (**self)
    }
}

impl<T, U: ?Sized> AsRef<U> for ArcCow<T>
where
    T: Borrow<U>,
{
    fn as_ref(&self) -> &U {
        (*self.0).borrow()
    }
}

impl<T: Clone, U: ?Sized> AsMut<U> for ArcCow<T>
where
    T: BorrowMut<U>,
{
    fn as_mut(&mut self) -> &mut U {
        (**self).borrow_mut()
    }
}

impl<T: Clone> ArcCow<T> {
    pub fn into_inner(self) -> T {
        Arc::try_unwrap(self.0).unwrap_or_else(|value| (*value).clone())
    }

    pub fn from_ref<U: ?Sized + ToOwned<Owned = T>>(src: &U) -> Self {
        Self::from(src.to_owned())
    }
}

impl<T> Deref for ArcCow<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for ArcCow<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

pub type ArcPath = ArcCow<PathBuf>;

impl From<&Path> for ArcPath {
    fn from(value: &Path) -> Self {
        Self::from_ref(value)
    }
}

impl From<&str> for ArcPath {
    fn from(value: &str) -> Self {
        Self::from_ref::<Path>(value.as_ref())
    }
}

impl From<String> for ArcPath {
    fn from(value: String) -> Self {
        Self::from_ref::<Path>(value.as_ref())
    }
}

impl From<&OsStr> for ArcPath {
    fn from(value: &OsStr) -> Self {
        Self::from_ref::<Path>(value.as_ref())
    }
}

impl From<OsString> for ArcPath {
    fn from(value: OsString) -> Self {
        Self::from_ref::<Path>(value.as_ref())
    }
}

impl From<ArcPath> for PathBuf {
    fn from(value: ArcPath) -> Self {
        value.into_inner()
    }
}
