use core::iter;

use gramma::parse::{Location, LocationRange};

use crate::utils::StrCursor;

pub struct EscapeError {
    pub range: LocationRange,
}

pub enum UnescapePart<'src> {
    Slice(&'src str),
    Char(char),
}

const ESCAPE_CHAR: char = '\\';

pub fn unescape_parts<'src>(
    slice: &'src str,
    slice_offset: impl Into<Option<Location>>,
) -> impl Iterator<Item = Result<UnescapePart<'src>, EscapeError>> + 'src {
    let slice_offset = slice_offset.into().unwrap_or(Location::MIN);
    let mut cursor = StrCursor::new(slice);

    iter::from_fn(move || {
        match cursor.advance_to_char(ESCAPE_CHAR) {
            Ok("") => {}
            Err("") => return None,
            Err(pre) | Ok(pre) => return Some(Ok(UnescapePart::Slice(pre))),
        }

        let start = slice_offset + cursor.position();
        let _ = cursor.next();
        let Some(first_char) = cursor.next() else {
            return Some(Err(EscapeError {
                range: LocationRange {
                    start,
                    end: slice_offset + cursor.position(),
                },
            }));
        };

        let out = match first_char {
            't' => Ok('\t'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            'x' => cursor
                .advance_by(2)
                .ok()
                .and_then(|digits| u8::from_str_radix(digits, 16).ok().filter(|&x| x <= 0x7f))
                .map(char::from)
                .ok_or(()),
            'u' => 'out: {
                if cursor.peek(0) != Some('{') {
                    break 'out Err(());
                };
                let _ = cursor.next();

                let digits = match cursor.advance_to_char('}') {
                    Ok(digits) if (1..=6).contains(&digits.len()) => digits,
                    _ => break 'out Err(()),
                };

                let _ = cursor.next();

                u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or(())
            }
            ch @ ('<' | '>' | '{' | '}' | '"' | '\'' | '\\' | '[' | ']') => Ok(ch),
            _ => Err(()),
        };

        Some(out.map(UnescapePart::Char).map_err(|()| EscapeError {
            range: LocationRange {
                start,
                end: slice_offset + cursor.position(),
            },
        }))
    })
}

pub fn escape_errors(
    slice: &str,
    slice_offset: Location,
) -> impl Iterator<Item = EscapeError> + '_ {
    unescape_parts(slice, slice_offset).filter_map(Result::err)
}
