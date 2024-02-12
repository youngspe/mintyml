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
