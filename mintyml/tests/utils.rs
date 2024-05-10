use core::fmt::{self, Write};

use mintyml::{ConvertError, OutputConfig};

#[track_caller]
fn convert_inner(
    src: &str,
    cfg: impl Into<Option<OutputConfig<'static>>>,
    forgive: bool,
) -> Result<String, (Option<String>, ConvertError)> {
    let cfg = cfg.into().unwrap_or_default();
    let res1 = mintyml::convert(src.as_ref(), &cfg);
    let res2 = mintyml::convert_forgiving(src.as_ref(), &cfg);

    match (res1, res2) {
        (Ok(x), Ok(y)) => {
            assert_eq!(x, y, "convert vs convert_forgiving");
            Ok(x)
        }
        (Ok(_), Err((_, e))) => panic!("convert succeeded but convert_forgiving yielded {e:?}"),
        (Err(e), Ok(_)) => panic!("convert yielded {e:?} but convert_forgiving succeeded"),
        (Err(_), Err(e)) if forgive => Err(e),
        (Err(e), Err(_)) => Err((None, e)),
    }
}

#[track_caller]
pub fn convert_unwrap(
    src: impl AsRef<str>,
    cfg: impl Into<Option<OutputConfig<'static>>>,
) -> String {
    match convert_inner(src.as_ref(), cfg, false) {
        Ok(x) => x,
        #[cfg(feature = "std")]
        Err((_, e)) => panic!("{e}"),
        #[cfg(not(feature = "std"))]
        Err((_, e)) => panic!("{e:?}"),
    }
}

#[track_caller]
pub fn convert_fail(
    src: &str,
    cfg: impl Into<Option<OutputConfig<'static>>>,
) -> (Option<String>, ConvertError) {
    match convert_inner(src, cfg, true) {
        Ok(_) => panic!("conversion expected to fail but succeeded"),
        Err(e) => e,
    }
}

pub fn match_set<T: fmt::Debug>(
    src: impl IntoIterator<Item = T>,
    mut matchers: Vec<(&mut dyn FnMut(&mut T) -> bool, &str)>,
) {
    let mut src = Vec::from_iter(src);
    src.retain_mut(|item| {
        match matchers
            .iter_mut()
            .enumerate()
            .filter_map(|(i, (f, _))| f(item).then_some(i))
            .next()
        {
            Some(i) => {
                matchers.remove(i);
                false
            }
            None => true,
        }
    });

    if src.is_empty() && matchers.is_empty() {
        return;
    }

    let mut msg = String::new();

    if !src.is_empty() {
        writeln!(msg, "Unexpected items:").unwrap();
        for item in src {
            writeln!(msg, "{item:?}").unwrap();
        }
    }

    if !matchers.is_empty() {
        writeln!(msg, "Expected items matching:").unwrap();
        for (_, desc) in matchers {
            writeln!(msg, "{desc}").unwrap();
        }
    }

    panic!("{msg}");
}

#[macro_export]
macro_rules! match_set {
    ($lhs:expr, [$(|$pat:pat_param| $check:expr),* $(,)?] $(,)?) => {
        $crate::utils::match_set($lhs, vec![
            $((&mut |__item| match *__item {
                $pat => $check,
                _ => false,
            }, concat!(stringify!($pat), " => ", stringify!($check))),)*
        ])
    };
}
