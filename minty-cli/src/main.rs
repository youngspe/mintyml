extern crate anyhow;
extern crate clap;
extern crate either;
extern crate mintyml;
extern crate rayon;
extern crate serde;
extern crate serde_json;

#[macro_use]
mod utils;
mod args;
mod convert;
mod error_reporter;
mod key_value;

use std::{
    fs::OpenOptions,
    io::{self, stderr, Read, Seek, Write},
    path::Path,
    process::ExitCode,
};

use anyhow::Result as AnyResult;
use clap::Parser;
use either::Either::{Left, Right};

use error_reporter::{ErrorReporter, StreamName, StreamNameRef};
use utils::UtilExt as _;

impl args::Cli {
    fn execute(self, cx: &mut AppCx<impl CxType>) -> AnyResult<bool> {
        match self.command {
            args::Command::Convert(convert) => convert.execute(cx),
        }
    }
}

trait CxType: Send + Sync {
    type Io: IoHelper;
}

trait IoHelper: Send + Sync {
    fn open_read_path(&self, path: &Path) -> AnyResult<impl Read + Seek + '_>;
    fn open_write_path(&self, path: &Path) -> AnyResult<impl Write + Seek + '_>;
    fn read<'path>(&self, name: impl Into<StreamNameRef<'path>>) -> AnyResult<String> {
        io::read_to_string(self.open_read(name)?).err_into()
    }
    fn write<'path>(&self, name: impl Into<StreamNameRef<'path>>, value: &str) -> AnyResult<()> {
        self.open_write(name)?
            .write_all(value.as_bytes())
            .err_into()
    }
    fn stdin(&self) -> AnyResult<impl Read + '_>;
    fn stdout(&self) -> AnyResult<impl Write + '_>;
    fn stderr(&self) -> AnyResult<impl Write + '_>;
    fn open_read<'path>(&self, name: impl Into<StreamNameRef<'path>>) -> AnyResult<impl Read + '_> {
        match name.into() {
            StreamName::File(path) => self.open_read_path(path).map(Left),
            StreamName::Stdio => self.stdin().map(Right),
        }
    }
    fn open_write<'path>(
        &self,
        name: impl Into<StreamNameRef<'path>>,
    ) -> AnyResult<impl Write + '_> {
        match name.into() {
            StreamName::File(path) => self.open_write_path(path).map(Left),
            StreamName::Stdio => self.stdout().map(Right),
        }
    }
}

struct DefaultCx;

impl CxType for DefaultCx {
    type Io = DefaultIoHelper;
}

struct DefaultIoHelper;

impl IoHelper for DefaultIoHelper {
    fn open_read_path(&self, path: &Path) -> AnyResult<impl Read + Seek + '_> {
        std::fs::File::open(path).err_into()
    }

    fn open_write_path(&self, path: &Path) -> AnyResult<impl Write + Seek + '_> {
        let f = OpenOptions::new().create(true).write(true).open(path)?;
        f.set_len(0)?;
        Ok(f)
    }

    fn stdin(&self) -> AnyResult<impl Read> {
        std::io::stdin().ok()
    }

    fn stdout(&self) -> AnyResult<impl Write> {
        std::io::stdout().ok()
    }

    fn stderr(&self) -> AnyResult<impl Write> {
        std::io::stdout().ok()
    }
}

struct AppCx<Cx: CxType> {
    #[allow(unused)]
    pub cx_type: Cx,
    pub io: Cx::Io,
    pub reporter: ErrorReporter,
}

fn main() -> ExitCode {
    let mut cx = AppCx {
        cx_type: DefaultCx,
        io: DefaultIoHelper,
        reporter: ErrorReporter::initialize(stderr()),
    };
    match args::Cli::parse().execute(&mut cx) {
        Err(e) => {
            cx.reporter.other_error(e);
            return ExitCode::FAILURE;
        }
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
    }
}
