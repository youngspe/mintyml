use mintyml::OutputConfig;

#[track_caller]
pub fn convert_unwrap(src: impl AsRef<str>, cfg: impl Into<Option<OutputConfig<'static>>>) -> String {
    let res = mintyml::convert(src.as_ref(), cfg.into().unwrap_or_default());
    #[cfg(feature = "std")]
    {
        match res {
            Ok(x) => x,
            Err(e) => panic!("{e}"),
        }
    }
    #[cfg(not(feature = "std"))]
    {
        res.unwrap()
    }
}
