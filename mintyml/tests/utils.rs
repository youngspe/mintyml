use mintyml::OutputConfig;

pub fn convert_unwrap(src: impl AsRef<str>, cfg: impl Into<Option<OutputConfig>>) -> String {
    let res = mintyml::convert(src.as_ref(), cfg.into().unwrap_or_default());
    #[cfg(feature = "std")]
    {
        res.unwrap_or_else(|e| panic!("{e}"))
    }
    #[cfg(not(feature = "std"))]
    {
        res.unwrap()
    }
}
