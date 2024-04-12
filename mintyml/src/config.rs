use crate::document::Src;

/// Defines overrides for the element types (or _tags_) inferred from special
/// inline elements and code blocks.
#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct SpecialTagConfig<'src> {
    /// Tag name for `</ ... />`. Defaults to `em`.
    pub emphasis: Option<Src<'src>>,
    /// Tag name for `<# ... #>`. Defaults to `strong`.
    pub strong: Option<Src<'src>>,
    /// Tag name for `<_ ... _>`. Defaults to `u`.
    pub underline: Option<Src<'src>>,
    /// Tag name for `<~ ... ~>`. Defaults to `s`.
    pub strike: Option<Src<'src>>,
    /// Tag name for `<" ... ">`. Defaults to `q`.
    pub quote: Option<Src<'src>>,
    /// Tag name for ``<` ... `>``. Defaults to `code`.
    pub code: Option<Src<'src>>,
    /// Code blocks (denoted with ```` ``` ````) will use the same tag as inline code,
    /// wrapped in the tag provided by this field. Defaults to `pre`.
    pub code_block_container: Option<Src<'src>>,
}

/// Configuration options for converting a MinTyML document.
#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct OutputConfig<'src> {
    /// If `Some(tab)`, `tab` will be inserted for each indentation level.
    /// The string should only contain whitespace in order to produce valid HTML.
    /// If `None`, the output will not automatically insert indentation or line breaks.
    pub indent: Option<Src<'src>>,
    /// Whether the output should be in XHTML5 rather than HTML.
    /// Defaults to `false`.
    pub xml: Option<bool>,
    /// Overrides for the tags that correspond to each kind of special element.
    pub special_tags: SpecialTagConfig<'src>,
    /// Whether the output should be a complete, valid HTML page. Defaults to `false`.
    ///
    /// See the documentation for [OutputConfig::lang] for semantics.
    pub complete_page: Option<bool>,
    /// If provided, this value will be assigned to the `lang` attribute of each top-level element.
    ///
    /// This is most useful when `complete_page` is enabled so that the root element has a `lang` attribute.
    pub lang: Option<Src<'src>>,
}

impl<'src> OutputConfig<'src> {
    /// Instantiates with default values for all options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience method for mutating `self` in the same expression where it was created.
    ///
    /// # Example
    /// ```
    /// # use mintyml::OutputConfig;
    /// let config = OutputConfig::new()
    ///     .update(|config| config.indent = Some("  ".into()))
    ///     .update(|config| config.xml = Some(true));
    ///
    /// assert_eq!(config.indent.as_deref(), Some("  "));
    /// assert_eq!(config.xml, Some(true));
    /// ```
    pub fn update(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Sets the string `tab` to insert for each indentation level of the output.
    /// The string should only contain whitespace in order to produce valid HTML.
    /// Setting this value enables automatic line breaks in the output.
    ///
    /// # Example
    /// ## Indent with spaces
    ///
    /// ```
    /// # use mintyml::OutputConfig;
    /// let out = mintyml::convert(r#"
    /// {
    ///     Hello, world!
    /// }
    /// "#, OutputConfig::new().indent("  ")).unwrap();
    ///
    /// assert_eq!(out, "\
    /// <div>
    ///   <p>Hello, world!</p>
    /// </div>
    /// ");
    /// ```
    ///
    /// ## Indent with tabs
    ///
    /// ```
    /// # use mintyml::OutputConfig;
    /// let out = mintyml::convert(r#"
    /// {
    ///     Hello, world!
    /// }
    /// "#, OutputConfig::new().indent("\t")).unwrap();
    ///
    /// assert_eq!(out, "\
    /// <div>
    /// \t<p>Hello, world!</p>
    /// </div>
    /// ");
    /// ```
    pub fn indent(self, tab: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.indent = Some(tab.into()))
    }

    /// Specifies whether the output document should be in XHTML5 rather than HTML.
    ///
    /// This is useful when the output needs to be read by an XML parser.
    pub fn xml(self, enabled: bool) -> Self {
        self.update(|c| c.xml = Some(enabled))
    }

    /// Overrides the tag used for `</ ... />`. Defaults to `em`.
    pub fn emphasis_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.emphasis = Some(tag.into()))
    }

    /// Overrides the tag used for `<# ... #>`. Defaults to `strong`.
    pub fn strong_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.strong = Some(tag.into()))
    }

    /// Overrides the tag used for `<_ ... _>`. Defaults to `u`.
    pub fn underline_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.underline = Some(tag.into()))
    }

    /// Overrides the tag used for `<~ ... ~>`. Defaults to `s`.
    pub fn strike_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.strike = Some(tag.into()))
    }

    /// Overrides the tag used for `<" ... ">`. Defaults to `q`.
    pub fn quote_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.quote = Some(tag.into()))
    }

    /// Overrides the tag used for ``<` ... `>``. Defaults to `code`.
    pub fn code_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.code = Some(tag.into()))
    }

    /// Code blocks (denoted with ```` ``` ````) will use the same tag as inline code,
    /// wrapped in the tag provided by this call. Defaults to `pre`.
    pub fn code_block_container_tag(self, tag: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.special_tags.code_block_container = Some(tag.into()))
    }

    /// Whether the output should be a complete, valid HTML page. Defaults to `false`.
    ///
    /// If enabled, the page will be automatically wrapped in `<html>` tags if one is not present.
    /// If a `<body>` element exists at the top level, all top-level elements will be placed directly within the
    /// `<html>` element.
    /// Otherwise, all elements except those that belong in a `<head>` are placed in a `<body>` element.
    /// The remaining elements will be merged into a `<head>` element.
    ///
    /// If disabled, the document structure will remain unchanged.
    pub fn complete_page(self, enabled: bool) -> Self {
        self.update(|c| c.complete_page = enabled.into())
    }

    /// If provided, this value will be assigned to the `lang` attribute of each top-level element.
    ///
    /// This is most useful when `complete_page` is enabled so that the root element has a `lang` attribute.
    pub fn lang(self, lang: impl Into<Src<'src>>) -> Self {
        self.update(|c| c.lang = lang.into().into())
    }
}
