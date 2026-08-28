//! `chdef-gen`: a definition file in, a constant table out.
//!
//! Reads a CH CSV and an optional BF CSV as the host does, and writes the
//! layout as Rust source, a C header, or both. A definition with any Issue
//! is refused with the findings on stderr and a non-zero status: a row the
//! host would load with a warning does not reach a device.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chdef::ColumnVocabulary;
use chdef_core::Endian;

const USAGE: &str = "usage: chdef-gen --ch <ch.csv> [--bf <bf.csv>] [--endian little|big] \
                     [--japanese] [--rust <out.rs>] [--c <out.h>]
at least one of --rust and --c is needed; the table is written to no file otherwise.
";

/// What the command line asked for.
struct Request {
    ch: PathBuf,
    bf: Option<PathBuf>,
    endian: Endian,
    japanese: bool,
    rust: Option<PathBuf>,
    c: Option<PathBuf>,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(request) = read_arguments(&arguments) else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    match generate(&request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// The request the arguments make, or `None` when they make none.
fn read_arguments(arguments: &[String]) -> Option<Request> {
    let mut request = Request {
        ch: PathBuf::new(),
        bf: None,
        endian: Endian::Little,
        japanese: false,
        rust: None,
        c: None,
    };
    let mut seen_ch = false;
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--japanese" => request.japanese = true,
            "--ch" => {
                request.ch = PathBuf::from(arguments.next()?);
                seen_ch = true;
            }
            "--bf" => request.bf = Some(PathBuf::from(arguments.next()?)),
            "--rust" => request.rust = Some(PathBuf::from(arguments.next()?)),
            "--c" => request.c = Some(PathBuf::from(arguments.next()?)),
            "--endian" => {
                request.endian = match arguments.next()?.as_str() {
                    "little" => Endian::Little,
                    "big" => Endian::Big,
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
    let asked_for_a_table = request.rust.is_some() || request.c.is_some();
    (seen_ch && asked_for_a_table).then_some(request)
}

/// The request carried out, or the one line saying why it was not.
fn generate(request: &Request) -> Result<(), String> {
    let ch = read(&request.ch)?;
    let bf = match &request.bf {
        Some(path) => read(path)?,
        None => Vec::new(),
    };
    let vocabulary = if request.japanese {
        ColumnVocabulary::japanese()
    } else {
        ColumnVocabulary::new()
    };

    let model = chdef_gen::model(&ch, &bf, request.endian, &vocabulary)
        .map_err(|refusal| refusal.to_string())?;

    let origin = match &request.bf {
        Some(bf) => format!("{} + {}", request.ch.display(), bf.display()),
        None => request.ch.display().to_string(),
    };
    if let Some(path) = &request.rust {
        write(path, &chdef_gen::rust_source(&model, &origin))?;
    }
    if let Some(path) = &request.c {
        write(path, &chdef_gen::c_header(&model, &origin))?;
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
