extern crate anyhow;
extern crate clap;
extern crate either;
extern crate mintyml;
extern crate rayon;
extern crate serde;
extern crate serde_json;
#[cfg(test)]
extern crate shlex;

#[macro_use]
mod utils;
mod args;
mod convert;
mod error_reporter;
mod io_helper;
mod key_value;
#[cfg(test)]
mod tests;

use std::process::ExitCode;

use clap::Parser;

use error_reporter::ErrorReporter;
use io_helper::IoHelper;

pub type Result<T = (), E = anyhow::Error> = core::result::Result<T, E>;

impl args::Cli {
    fn execute(self, cx: &mut AppCx<impl CxType>) -> Result<bool> {
        match self.command {
            args::Command::Convert(convert) => convert.execute(cx),
        }
    }
}

trait CxType: Send + Sync {
    type Io: IoHelper + 'static;
}
struct DefaultCx;

impl CxType for DefaultCx {
    type Io = io_helper::DefaultIoHelper;
}

#[cfg(test)]
struct TestCx;

#[cfg(test)]
impl CxType for TestCx {
    type Io = io_helper::test_helper::TestIoHelper;
}

struct AppCx<Cx: CxType> {
    #[allow(unused)]
    pub cx_type: Cx,
    pub io: Cx::Io,
    pub reporter: ErrorReporter,
}

impl<Cx: CxType> AppCx<Cx> {
    fn new(cx_type: Cx, io: Cx::Io) -> Self {
        Self {
            cx_type,
            reporter: ErrorReporter::initialize(io.clone()),
            io,
        }
    }
}

#[cfg(test)]
struct TestOutput {
    pub outcome: Result<bool>,
    pub stdout: String,
    pub stderr: String,
    pub root: io_helper::test_helper::TestDir,
}

#[cfg(test)]
fn test_main<'arg>(
    args: impl AsRef<str>,
    input: impl Into<Option<&'arg str>>,
    root: impl Into<Option<io_helper::test_helper::TestDir>>,
    cwd: impl Into<Option<&'arg str>>,
) -> TestOutput {
    use utils::UtilExt;
    let args = args.as_ref();

    let root = root.into().unwrap_or_default();
    let io = io_helper::test_helper::TestIoHelper::new(
        root.clone(),
        cwd.into().unwrap_or("/").into(),
        input.into().unwrap_or(""),
    );
    let mut cx = AppCx::new(TestCx, io);
    let args = shlex::split(args).unwrap_or_else(|| panic!("Invalid arguments {args:?}"));
    let outcome = args::Cli::try_parse_from(args)
        .err_into()
        .and_then(|cli| cli.execute(&mut cx));

    let stdout = cx.io.stdout();
    let stderr = cx.io.stderr();

    drop(cx);

    let stdout = stdout.into_inner();
    let stderr = stderr.into_inner();

    TestOutput {
        outcome,
        stdout: String::from_utf8(stdout).unwrap_or_else(|e| e.to_string()),
        stderr: String::from_utf8(stderr).unwrap_or_else(|e| e.to_string()),
        root,
    }
}

fn parse_args(cx: &AppCx<impl CxType>) -> Option<args::Cli> {
    #[derive(clap::Parser)]
    struct FallbackArgs {
        #[arg(long, default_value = "default")]
        error_mode: args::ErrorMode,
    }

    match args::Cli::try_parse() {
        Ok(cli) => Some(cli),
        Err(e) => match FallbackArgs::try_parse() {
            Ok(FallbackArgs {
                error_mode: args::ErrorMode::Default,
            })
            | Err(_) => e.exit(),
            Ok(args) => {
                cx.reporter.set_mode(args.error_mode);
                None
            }
        },
    }
}

fn main() -> ExitCode {
    let mut cx = AppCx::new(DefaultCx, io_helper::DefaultIoHelper);
    let Some(cli) = parse_args(&cx) else {
        return ExitCode::FAILURE;
    };
    match cli.execute(&mut cx) {
        Err(e) => {
            cx.reporter.other_error(e);
            return ExitCode::FAILURE;
        }
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
    }
}
