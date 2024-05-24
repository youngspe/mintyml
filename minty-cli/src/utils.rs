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

    fn ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    fn some(self) -> Option<Self> {
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

    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
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
