use std::ffi::OsStr;

use clap::builder::{PossibleValue, TypedValueParser, ValueParserFactory};

#[derive(Clone)]
pub struct KeyValueParser<K, V> {
    key_parser: K,
    value_parser: V,
}

impl<K: TypedValueParser, V: TypedValueParser> KeyValueParser<K, V> {
    #[allow(unused)]
    pub fn new<K2, V2>() -> Self
    where
        K2: ValueParserFactory<Parser = K>,
        V2: ValueParserFactory<Parser = V>,
    {
        Self::with_parsers(K2::value_parser(), V2::value_parser())
    }

    pub fn with_parsers(key_parser: K, value_parser: V) -> Self {
        Self {
            key_parser,
            value_parser,
        }
    }
}

impl<K, V> Default for KeyValueParser<K, V>
where
    K: TypedValueParser + Default,
    V: TypedValueParser + Default,
{
    fn default() -> Self {
        Self::with_parsers(K::default(), V::default())
    }
}

impl<K: TypedValueParser, V: TypedValueParser> TypedValueParser for KeyValueParser<K, V> {
    type Value = (K::Value, V::Value);

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        src_str: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let src_str = src_str.to_string_lossy();

        let Some((key, value)) = src_str.split_once('=') else {
            use clap::error::{ContextKind, ContextValue, Error, ErrorKind};
            let mut error = Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
            if let Some(name) = arg.map(|arg| arg.to_string()) {
                error.insert(ContextKind::InvalidArg, ContextValue::String(name));
            }
            error.insert(
                ContextKind::InvalidValue,
                ContextValue::String(src_str.into_owned()),
            );
            error.insert(
                ContextKind::ValidValue,
                if let Some(possible_values) = self.possible_values() {
                    let mut strings = Vec::new();

                    possible_values.for_each(|p| {
                        strings.extend(p.get_name_and_aliases().map(Into::into));
                    });

                    ContextValue::Strings(strings)
                } else {
                    ContextValue::Strings(vec!["<KEY>=<VALUE>".into()])
                },
            );
            error.insert(ContextKind::Custom, ContextValue::String("foo".into()));
            return Err(error);
        };

        let key = self.key_parser.parse_ref(cmd, arg, key.as_ref())?;
        let value = self.value_parser.parse_ref(cmd, arg, value.as_ref())?;

        Ok((key, value))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        let possible_keys = self.key_parser.possible_values();
        let possible_values = self.value_parser.possible_values();

        if possible_keys.is_none() && possible_values.is_none() {
            return None;
        }

        let possible_keys =
            map_possible_values(possible_keys.into_iter().flatten(), |s| format!("{s}=..."));

        let possible_values = map_possible_values(possible_values.into_iter().flatten(), |s| {
            format!("...={s}")
        });

        Some(Box::new(possible_keys.chain(possible_values)))
    }
}

fn map_possible_values<'a>(
    p: impl IntoIterator<Item = PossibleValue> + 'a,
    mut f: impl FnMut(&str) -> String + 'a,
) -> impl Iterator<Item = PossibleValue> + 'a {
    p.into_iter().map(move |p| {
        let mut name_aliases = p.get_name_and_aliases().map(&mut f);

        let Some(name) = name_aliases.next() else {
            drop(name_aliases);
            return p;
        };

        let mut p2 = PossibleValue::new(name)
            .aliases(name_aliases)
            .hide(p.is_hide_set());
        if let Some(help) = p.get_help() {
            p2 = p2.help(help.clone());
        }
        p2
    })
}
