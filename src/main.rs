//! `word_counter` — count whitespace-separated words (or, with `--letters`,
//! alphabetic characters) in text files or on standard input.
//!
//! Exit codes: `0` success, `1` at least one input could not be counted,
//! `2` invalid command line.

use std::env;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use word_counter::{count_letters, count_letters_in_file, count_words, count_words_in_file};

const USAGE: &str = "\
Usage: word_counter [OPTION]... [FILE]...
Count whitespace-separated words in each FILE (or, with -l, alphabetic
characters) and print one result per line.

With no FILE, or when FILE is -, read standard input.

Options:
  -l, --letters    count alphabetic characters instead of words
  -h, --help       display this help and exit
  -V, --version    output version information and exit
";

const PROGRAM: &str = "word_counter";
const EXIT_USAGE: u8 = 2;

/// What the command line asked the program to do.
enum Action {
    Count { inputs: Vec<String>, letters: bool },
    Help,
    Version,
}

fn main() -> ExitCode {
    let action = match parse_args(env::args().skip(1)) {
        Ok(action) => action,
        Err(message) => {
            eprintln!("{PROGRAM}: {message}");
            eprintln!("Try '{PROGRAM} --help' for more information.");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    match action {
        Action::Help => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Action::Version => {
            println!("{PROGRAM} {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Action::Count { inputs, letters } => count(inputs, letters),
    }
}

/// Translates raw arguments into an [`Action`], rejecting unknown options.
fn parse_args(args: impl Iterator<Item = String>) -> Result<Action, String> {
    let mut inputs = Vec::new();
    let mut letters = false;
    let mut literal = false;

    for arg in args {
        if literal {
            inputs.push(arg);
            continue;
        }
        match arg.as_str() {
            "--" => literal = true,
            "-h" | "--help" => return Ok(Action::Help),
            "-V" | "--version" => return Ok(Action::Version),
            "-l" | "--letters" => letters = true,
            // A lone "-" is the conventional name for standard input.
            "-" => inputs.push(arg),
            option if option.starts_with('-') => {
                return Err(format!("unrecognized option '{option}'"));
            }
            _ => inputs.push(arg),
        }
    }

    Ok(Action::Count { inputs, letters })
}

/// Counts every requested input, reports failures on stderr, and prints the
/// successful results (plus a total row when there is more than one).
fn count(inputs: Vec<String>, letters: bool) -> ExitCode {
    let targets = if inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        inputs
    };

    let stdin = io::stdin();
    let mut results: Vec<(usize, String)> = Vec::new();
    let mut failed = false;

    for target in targets {
        let outcome = match (target.as_str(), letters) {
            ("-", false) => count_words(&mut stdin.lock()).map(|n| (n, "-".to_string())),
            ("-", true) => count_letters(&mut stdin.lock()).map(|n| (n, "-".to_string())),
            (path, false) => count_words_in_file(Path::new(path)).map(|n| (n, path.to_string())),
            (path, true) => count_letters_in_file(Path::new(path)).map(|n| (n, path.to_string())),
        };

        match outcome {
            Ok(result) => results.push(result),
            Err(err) => {
                eprintln!("{PROGRAM}: {err}");
                failed = true;
            }
        }
    }

    print_results(&results);

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_results(results: &[(usize, String)]) {
    let mut width = 1;
    for (count, _) in results {
        width = width.max(count.to_string().len());
    }

    let show_total = results.len() > 1;
    let total: usize = results.iter().map(|(count, _)| count).sum();
    if show_total {
        width = width.max(total.to_string().len());
    }

    for (count, label) in results {
        println!("{count:>width$}  {label}");
    }
    if show_total {
        println!("{total:>width$}  total");
    }
}
