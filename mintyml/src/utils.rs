use core::{fmt, iter};

pub fn default<T: Default>() -> T {
    T::default()
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrCursor<'src> {
    position: usize,
    src: &'src str,
}

impl<'src> StrCursor<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { src, position: 0 }
    }

    pub fn src(&self) -> &'src str {
        self.src
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn split(&self) -> (&'src str, &'src str) {
        self.src.split_at(self.position)
    }

    pub fn pre(&self) -> &'src str {
        self.src.get(..self.position).unwrap_or("")
    }

    pub fn post(&self) -> &'src str {
        self.src.get(self.position..).unwrap_or("")
    }

    pub fn jump_to_end(&mut self) -> &'src str {
        let slice = self.post();
        self.position = self.src.len();
        slice
    }

    pub fn advance_by(&mut self, num: usize) -> Result<&'src str, &'src str> {
        match self.src.char_indices().take(num).enumerate().last() {
            Some((n, (i, ch))) => {
                let len = i + ch.len_utf8();
                let slice = self
                    .src
                    .get(self.position..self.position + len)
                    .unwrap_or("");
                if n == num - 1 {
                    Ok(slice)
                } else {
                    Err(slice)
                }
            }
            None => Ok(""),
        }
    }

    pub fn advance_to_char(&mut self, p: char) -> Result<&'src str, &'src str> {
        let post = self.post();
        match post.find(p) {
            Some(dist) => {
                self.position += dist;
                Ok(post.get(..dist).unwrap_or(""))
            }
            None => {
                self.position = self.src.len();
                Err(post)
            }
        }
    }

    pub fn next(&mut self) -> Option<char> {
        let ch = self.post().chars().next()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    pub fn peek(&self, dist: usize) -> Option<char> {
        self.post().chars().nth(dist)
    }
}

impl<'src> From<&'src str> for StrCursor<'src> {
    fn from(src: &'src str) -> Self {
        Self::new(src)
    }
}

pub struct DisplayFn<F: Fn(&mut fmt::Formatter) -> fmt::Result>(pub F);

impl<F> fmt::Display for DisplayFn<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0(f)
    }
}

impl<F> fmt::Debug for DisplayFn<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0(f)
    }
}

pub fn join_fmt<T>(
    src: impl IntoIterator<Item = T> + Clone,
    fmt: impl Fn(T, &mut fmt::Formatter) -> fmt::Result,
    sep: impl fmt::Display,
) -> impl fmt::Display {
    DisplayFn(move |f| {
        let src = src.clone();
        let mut iter = src.into_iter();
        let Some(first) = iter.next() else {
            return Ok(());
        };

        fmt(first, f)?;
        iter.try_for_each(|x| {
            sep.fmt(f)?;
            fmt(x, f)
        })
    })
}

pub fn join_display<T: fmt::Display>(
    src: impl IntoIterator<Item = T> + Clone,
    sep: impl fmt::Display,
) -> impl fmt::Display {
    join_fmt(src, |x, f| fmt::Display::fmt(&x, f), sep)
}

pub fn intersperse_with<T>(
    iter: impl IntoIterator<Item = T>,
    f: impl FnMut() -> T,
) -> impl Iterator<Item = T> {
    let mut iter = iter.into_iter();
    let first = iter.next();

    first
        .into_iter()
        .chain(iter::zip(iter::repeat_with(f), iter).flat_map(|(item1, item2)| [item1, item2]))
}

pub fn try_extend<T, E>(
    out: &mut impl Extend<T>,
    src: impl IntoIterator<Item = Result<T, E>>,
) -> Result<(), E> {
    src.into_iter().try_for_each(|item| item.map(|x| out.extend([x])))
}
