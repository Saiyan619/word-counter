//! End-to-end tests that drive the compiled `word_counter` binary.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_word_counter");
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hello.txt");

/// Runs the binary with `args`, optionally feeding `stdin`, and captures both
/// output streams.
fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("failed to spawn word_counter");

    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().expect("missing child stdin pipe");
        pipe.write_all(input.as_bytes())
            .expect("failed to write to child stdin");
    }

    child
        .wait_with_output()
        .expect("failed to wait for word_counter")
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout was not UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr was not UTF-8")
}

#[test]
fn counts_words_in_a_file() {
    let output = run(&[FIXTURE], None);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("14  {FIXTURE}\n"));
}

#[test]
fn reads_stdin_when_no_file_is_given() {
    let output = run(&[], Some("the quick   brown fox\n\njumps\n"));
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "5  -\n");
}

#[test]
fn explicit_dash_reads_stdin() {
    let output = run(&["-"], Some("one two three"));
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "3  -\n");
}

#[test]
fn prints_a_total_row_for_multiple_files() {
    let output = run(&[FIXTURE, FIXTURE], None);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let expected = format!("14  {FIXTURE}\n14  {FIXTURE}\n28  total\n");
    assert_eq!(stdout(&output), expected);
}

#[test]
fn missing_file_fails_on_stderr_without_stdout() {
    let output = run(&["definitely/not/a/real/file.txt"], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty(), "stdout: {}", stdout(&output));
    assert!(
        stderr(&output).contains("cannot open definitely/not/a/real/file.txt"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn counts_letters_instead_of_words_when_requested() {
    let output = run(&["--letters", FIXTURE], None);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("53  {FIXTURE}\n"));
}

#[test]
fn short_letters_flag_reads_stdin() {
    let output = run(&["-l"], Some("Hello, World! 42"));
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "10  -\n");
}

#[test]
fn letter_totals_are_aligned_across_files() {
    let output = run(&["-l", FIXTURE, FIXTURE], None);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let expected = format!(" 53  {FIXTURE}\n 53  {FIXTURE}\n106  total\n");
    assert_eq!(stdout(&output), expected);
}

#[test]
fn help_is_printed_to_stdout() {
    let output = run(&["--help"], None);
    assert!(output.status.success());
    assert!(stdout(&output).starts_with("Usage: word_counter"));
    assert!(stdout(&output).contains("-l, --letters"));
}

#[test]
fn version_matches_the_package_version() {
    let output = run(&["--version"], None);
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        format!("word_counter {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_option_is_a_usage_error() {
    let output = run(&["--bogus"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty(), "stdout: {}", stdout(&output));
    assert!(
        stderr(&output).contains("unrecognized option '--bogus'"),
        "stderr: {}",
        stderr(&output)
    );
}
