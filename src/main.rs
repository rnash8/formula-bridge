mod convert;
mod dialect;
mod lexer;

use convert::Mode;
use dialect::Dialect;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

struct Args {
    from: Dialect,
    to: Dialect,
    lenient: bool,
    input_path: Option<String>,
}

fn print_usage() {
    eprintln!("formula-bridge --from <excel|odf> --to <excel|odf> [--lenient] [FILE]");
    eprintln!();
    eprintln!("Converts spreadsheet formulas between Excel A1 syntax and ODF");
    eprintln!("(OpenDocument / LibreOffice Calc) formula syntax.");
    eprintln!();
    eprintln!("Reads one formula per line from FILE, or stdin if FILE is omitted.");
    eprintln!("A leading '=' on input is optional and always present on output.");
    eprintln!();
    eprintln!("By default unknown functions are rejected (strict mode). Pass");
    eprintln!("--lenient to pass unrecognized function names through unchanged.");
}

fn parse_args() -> Result<Args, String> {
    let mut from = None;
    let mut to = None;
    let mut lenient = false;
    let mut input_path = None;

    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "--from" => {
                let v = it.next().ok_or("--from requires a value")?;
                from = Some(Dialect::parse(&v)?);
            }
            "--to" => {
                let v = it.next().ok_or("--to requires a value")?;
                to = Some(Dialect::parse(&v)?);
            }
            "--lenient" => lenient = true,
            other if !other.starts_with('-') && input_path.is_none() => {
                input_path = Some(other.to_string());
            }
            other => return Err(format!("unrecognized argument '{}'", other)),
        }
    }

    let from = from.ok_or("missing required --from <excel|odf>")?;
    let to = to.ok_or("missing required --to <excel|odf>")?;
    Ok(Args { from, to, lenient, input_path })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}", e);
            print_usage();
            return ExitCode::from(2);
        }
    };

    let text = match &args.input_path {
        Some(path) => match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: could not read '{}': {}", path, e);
                return ExitCode::from(2);
            }
        },
        None => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                eprintln!("error: could not read stdin: {}", e);
                return ExitCode::from(2);
            }
            buf
        }
    };

    let mode = if args.lenient { Mode::Lenient } else { Mode::Strict };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let formula = trimmed.strip_prefix('=').unwrap_or(trimmed);
        match convert::convert(formula, args.from, args.to, mode) {
            Ok(converted) => {
                let _ = writeln!(out, "={}", converted);
            }
            Err(e) => {
                had_error = true;
                eprintln!("line {}: {}", i + 1, e.message);
            }
        }
    }

    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
