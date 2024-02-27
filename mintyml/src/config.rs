use alloc::borrow::Cow;

#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct SpecialTagConfig<'src> {
    pub emphasis: Option<Cow<'src, str>>,
    pub strong: Option<Cow<'src, str>>,
    pub underline: Option<Cow<'src, str>>,
    pub strike: Option<Cow<'src, str>>,
    pub quote: Option<Cow<'src, str>>,
    pub code: Option<Cow<'src, str>>,
    pub code_block_container: Option<Cow<'src, str>>,
}

#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct OutputConfig<'src> {
    pub indent: Option<Cow<'src, str>>,
    pub xml: Option<bool>,
    pub special_tags: SpecialTagConfig<'src>,
    pub complete_page: Option<bool>,
    pub lang: Option<Cow<'src, str>>,
}

impl<'src> OutputConfig<'src> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    pub fn indent(self, indent: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.indent = Some(indent.into()))
    }

    pub fn xml(self, xml: bool) -> Self {
        self.update(|c| c.xml = Some(xml))
    }

    pub fn emphasis_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.emphasis = Some(tag.into()))
    }

    pub fn strong_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.strong = Some(tag.into()))
    }

    pub fn underline_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.underline = Some(tag.into()))
    }

    pub fn strike_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.strike = Some(tag.into()))
    }

    pub fn quote_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.quote = Some(tag.into()))
    }

    pub fn code_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.code = Some(tag.into()))
    }

    pub fn code_block_container_tag(self, tag: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.special_tags.code_block_container = Some(tag.into()))
    }

    pub fn complete_page(self, complete_page: bool) -> Self {
        self.update(|c| c.complete_page = complete_page.into())
    }

    pub fn lang(self, lang: impl Into<Cow<'src, str>>) -> Self {
        self.update(|c| c.lang = lang.into().into())
    }
}
