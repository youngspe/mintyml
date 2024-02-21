pub fn trim_multiline<'src>(src: &'src str) -> impl Iterator<Item = &'src str> {
    src.split_once('\n')
        .and_then(|(_, src)| src.rsplit_once('\n'))
        .and_then(|(src, last)| {
            let prefix = last
                .find(|c: char| !c.is_whitespace())
                .and_then(|len| last.get(..len))
                .unwrap_or(last);

            src.strip_suffix('\r')
                .unwrap_or(src)
                .split_inclusive('\n')
                .map(move |l| l.strip_prefix(prefix).unwrap_or(l))
                .into()
        })
        .into_iter()
        .flatten()
}
