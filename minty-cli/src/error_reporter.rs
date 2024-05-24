use std::{
    borrow::Borrow,
    fmt::{self, Display},
    io::Write,
    ops::Deref,
    panic::panic_any,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    thread,
};

use mintyml::error::{DisplayWithSrcOptions, LocationRange};
use serde::{ser::Serializer, Serialize};

use crate::{args::ErrorMode, utils::default};

#[derive(Serialize)]
#[serde(bound = "M: Display, E: Iterator + Clone, E::Item: Display")]
#[serde(rename_all = "camelCase")]
struct ErrorEntry<'lt, M, E> {
    category: ErrorCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_name: Option<StreamName<&'lt Path>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<ErrorLocation>,
    #[serde(skip_serializing_if = "should_skip_iter")]
    #[serde(serialize_with = "serialize_display_iter")]
    expected: E,
    message: Option<SerializeDisplay<M>>,
}

fn should_skip_iter(it: &impl Iterator) -> bool {
    it.size_hint().1 == Some(0)
}

fn serialize_display_iter<S: Serializer, It: Iterator + Clone>(
    it: &It,
    s: S,
) -> Result<S::Ok, S::Error>
where
    It::Item: Display,
{
    s.collect_seq(it.clone().map(SerializeDisplay))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorLocation {
    start: usize,
    end: usize,
}

impl ErrorLocation {
    fn new(range: LocationRange) -> Self {
        Self {
            start: range.start.position,
            end: range.end.position,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SerializeDisplay<T>(pub T);

impl<T: Display> Serialize for SerializeDisplay<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum StreamName<S = Arc<PathBuf>> {
    File(S),
    Stdio,
}

pub type OwnedStreamName = StreamName;

pub type StreamNameRef<'path> = StreamName<&'path Path>;

impl<S> Display for StreamName<S>
where
    S: Deref,
    S::Target: Borrow<Path>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StreamName::File(name) => write!(f, "{}", (**name).borrow().display()),
            StreamName::Stdio => f.write_str("<stdio>"),
        }
    }
}

impl<S> StreamName<S> {
    pub fn as_ref(&self) -> StreamName<&Path>
    where
        S: Deref,
        S::Target: Borrow<Path>,
    {
        match self {
            StreamName::File(name) => StreamName::File((**name).borrow()),
            StreamName::Stdio => StreamName::Stdio,
        }
    }
}

impl<'path, S> From<&'path S> for StreamNameRef<'path>
where
    S: Deref,
    S::Target: Borrow<Path>,
{
    fn from(value: &'path S) -> Self {
        Self::File((**value).borrow())
    }
}

impl<'path> From<&'path Path> for StreamNameRef<'path> {
    fn from(value: &'path Path) -> Self {
        StreamName::File(value)
    }
}

impl<'path, S> From<&'path StreamName<S>> for StreamNameRef<'path>
where
    S: Deref,
    S::Target: Borrow<Path>,
{
    fn from(value: &'path StreamName<S>) -> Self {
        value.as_ref()
    }
}

struct ErrorEventData {
    src: Option<Arc<String>>,
    source_name: Option<StreamName>,
    syntax_errors: Vec<mintyml::error::SyntaxError>,
    semantic_errors: Vec<mintyml::error::SemanticError>,
    other_errors: Vec<anyhow::Error>,
}

impl ErrorEventData {
    fn report(self, out: &mut impl Write, mode: ErrorMode) -> anyhow::Result<()> {
        let Self {
            src,
            source_name,
            syntax_errors,
            semantic_errors,
            other_errors,
        } = self;

        let mut display_with_src_options = DisplayWithSrcOptions::default();
        display_with_src_options.show_location = false;

        for error in syntax_errors {
            let source_name = source_name.as_ref().map(|s| s.as_ref());
            let expected = match error.kind {
                mintyml::error::SyntaxErrorKind::ParseFailed { ref expected, .. } => Some(expected),
                _ => None,
            }
            .into_iter()
            .flatten();

            write_error(
                out,
                mode,
                ErrorEntry {
                    category: ErrorCategory::Syntax,
                    source_name,
                    location: ErrorLocation::new(error.range).into(),
                    expected,
                    message: src.as_ref().map(|s| {
                        SerializeDisplay(error.display_with_src(s, &display_with_src_options))
                    }),
                },
            )?;
        }
        for error in semantic_errors {
            let source_name = source_name.as_ref().map(|s| s.as_ref());
            write_error(
                out,
                mode,
                ErrorEntry {
                    category: ErrorCategory::Semantic,
                    source_name,
                    location: ErrorLocation::new(error.range).into(),
                    expected: std::iter::empty::<&str>(),
                    message: src.as_ref().map(|s| {
                        SerializeDisplay(error.display_with_src(s, &display_with_src_options))
                    }),
                },
            )?;
        }
        for error in other_errors {
            let category = error
                .downcast_ref::<ErrorCategory>()
                .copied()
                .unwrap_or(ErrorCategory::Internal);

            let source_name = error.downcast_ref::<StreamName>().map(StreamName::as_ref);

            write_error(
                out,
                mode,
                ErrorEntry {
                    category,
                    source_name,
                    location: None,
                    expected: std::iter::empty::<&str>(),
                    message: Some(SerializeDisplay(error.root_cause())),
                },
            )?;
        }
        Ok(())
    }
}

enum ErrorEvent {
    Error(ErrorEventData),
    Mode(ErrorMode),
    Close,
}

pub struct ErrorReporter {
    sender: mpsc::Sender<ErrorEvent>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for ErrorReporter {
    fn drop(&mut self) {
        self.sender.send(ErrorEvent::Close).unwrap();
        self.join_handle
            .take()
            .map(|j| j.join())
            .transpose()
            .unwrap_or_else(|e| panic_any(e));
    }
}

#[derive(Serialize, Debug, derive_more::Display, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCategory {
    Syntax,
    Semantic,
    Internal,
    Argument,
    Hidden,
}

fn write_error<'lt, M: Display, E: Iterator + Clone>(
    out: &mut impl Write,
    error_mode: ErrorMode,
    entry: ErrorEntry<'lt, M, E>,
) -> anyhow::Result<()>
where
    E::Item: Display,
{
    if matches!(entry.category, ErrorCategory::Hidden) {
        return Ok(());
    }

    match error_mode {
        ErrorMode::Default => {
            match entry.category {
                ErrorCategory::Argument => (),
                category => write!(out, "{category} error: ")?,
            }
            match entry.source_name {
                Some(StreamName::File(ref s)) => {
                    write!(out, "{}: ", s.display())?;
                }
                Some(StreamName::Stdio) => {
                    write!(out, "<stdin>: ")?;
                }
                _ => (),
            }
            if let Some(ErrorLocation { start, end }) = entry.location {
                write!(out, "at position {start}")?;
                if end >= start + 1 {
                    write!(out, "..<{end}")?;
                }
                write!(out, ": ")?;
            }
            if let Some(SerializeDisplay(ref message)) = entry.message {
                write!(out, "{message}")?;
            } else {
                write!(out, "unknown error")?;
            }
            write!(out, "\n")?;
        }
        ErrorMode::Json => {
            serde_json::to_writer(&mut *out, &entry)?;
            out.write_all(b"\n")?;
        }
        ErrorMode::Silent => {}
    }
    Ok(())
}

impl ErrorReporter {
    pub fn initialize(mut out: impl Write + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel::<ErrorEvent>();

        let handle = thread::spawn(move || {
            // batch up errors until we get a Mode or Close event
            let mut pending = Some(Vec::new());
            let mut mode = ErrorMode::Default;

            for event in receiver {
                let mut write_pending = false;
                let mut exit = false;

                match event {
                    ErrorEvent::Error(data) => match pending {
                        Some(ref mut batch) => {
                            batch.push(data);
                        }
                        None => data.report(&mut out, mode).unwrap(),
                    },
                    ErrorEvent::Mode(new_mode) => {
                        mode = new_mode;
                        write_pending = true;
                    }
                    ErrorEvent::Close => {
                        exit = true;
                        write_pending = true;
                    }
                }

                if write_pending {
                    pending
                        .take()
                        .unwrap_or_default()
                        .into_iter()
                        .try_for_each(|d| d.report(&mut out, mode))
                        .unwrap()
                }

                if exit {
                    break;
                }
            }
        });

        Self {
            sender,
            join_handle: Some(handle),
        }
    }

    pub fn conversion_error(&self, source_name: StreamName, error: mintyml::error::ConvertError) {
        let (src, syntax_errors, semantic_errors) = match error {
            mintyml::ConvertError::Syntax { syntax_errors, src } => {
                (Some(src), syntax_errors, default())
            }
            mintyml::ConvertError::Semantic {
                semantic_errors,
                src,
            } => (Some(src), default(), semantic_errors),
            _ => (None, default(), default()),
        };

        let src = src.map(|s| s.into_owned().into());

        self.sender
            .send(ErrorEvent::Error(ErrorEventData {
                src,
                source_name: Some(source_name),
                syntax_errors,
                semantic_errors,
                other_errors: Vec::new(),
            }))
            .unwrap()
    }

    pub fn other_error(&self, error: anyhow::Error) {
        self.sender
            .send(ErrorEvent::Error(ErrorEventData {
                src: None,
                source_name: None,
                syntax_errors: Vec::new(),
                semantic_errors: Vec::new(),
                other_errors: vec![error],
            }))
            .unwrap();
    }

    pub fn set_mode(&self, mode: ErrorMode) {
        self.sender.send(ErrorEvent::Mode(mode)).unwrap();
    }
}
