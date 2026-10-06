//! Core word-counting logic for the `word_counter` CLI.
//!
//! The counting routines are generic over [`BufRead`], so the same code can
//! serve files, standard input, and in-memory buffers. Input is streamed line
//! by line into a reused buffer, so memory use stays proportional to the
//! longest line instead of the whole file.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

/// An error raised while opening or reading an input source.
#[derive(Debug)]
pub enum Error {
    /// The file could not be opened.
    Open { path: PathBuf, source: io::Error },
    /// An already-opened source could not be read. The path is `None` when the
    /// source is standard input. Non-UTF-8 input surfaces here as well.
    Read {
        path: Option<PathBuf>,
        source: io::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Open { path, source } => {
                write!(f, "cannot open {}: {source}", path.display())
            }
            Error::Read {
                path: Some(path),
                source,
            } => write!(f, "cannot read {}: {source}", path.display()),
            Error::Read { path: None, source } => {
                write!(f, "cannot read standard input: {source}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Open { source, .. } | Error::Read { source, .. } => Some(source),
        }
    }
}

/// What to count in an input.
#[derive(Debug, Clone, Copy)]
enum Metric {
    /// Whitespace-separated words.
    Words,
    /// Alphabetic characters, per Unicode ([`char::is_alphabetic`]). Digits,
    /// punctuation, and whitespace are not letters.
    Letters,
}

impl Metric {
    fn count_line(self, line: &str) -> usize {
        match self {
            Metric::Words => line.split_whitespace().count(),
            Metric::Letters => line.chars().filter(|c| c.is_alphabetic()).count(),
        }
    }
}

/// Counts whitespace-separated words in `text`.
pub fn count_words_in_text(text: &str) -> usize {
    count_in_text(text, Metric::Words)
}

/// Counts alphabetic characters in `text`.
pub fn count_letters_in_text(text: &str) -> usize {
    count_in_text(text, Metric::Letters)
}

fn count_in_text(text: &str, metric: Metric) -> usize {
    // A line break is whitespace, so it can never split a word.
    text.lines().map(|line| metric.count_line(line)).sum()
}

/// Counts whitespace-separated words read from `reader`.
///
/// # Errors
///
/// Returns [`Error::Read`] if the reader fails or its content is not valid
/// UTF-8.
pub fn count_words<R: BufRead>(reader: &mut R) -> Result<usize, Error> {
    count_impl(reader, None, Metric::Words)
}

/// Counts alphabetic characters read from `reader`.
///
/// # Errors
///
/// Returns [`Error::Read`] if the reader fails or its content is not valid
/// UTF-8.
pub fn count_letters<R: BufRead>(reader: &mut R) -> Result<usize, Error> {
    count_impl(reader, None, Metric::Letters)
}

/// Opens `path` and counts the whitespace-separated words it contains.
///
/// # Errors
///
/// Returns [`Error::Open`] if the file cannot be opened, or [`Error::Read`]
/// if reading fails or the file is not valid UTF-8 text.
pub fn count_words_in_file(path: &Path) -> Result<usize, Error> {
    count_in_file(path, Metric::Words)
}

/// Opens `path` and counts the alphabetic characters it contains.
///
/// # Errors
///
/// Returns [`Error::Open`] if the file cannot be opened, or [`Error::Read`]
/// if reading fails or the file is not valid UTF-8 text.
pub fn count_letters_in_file(path: &Path) -> Result<usize, Error> {
    count_in_file(path, Metric::Letters)
}

fn count_in_file(path: &Path, metric: Metric) -> Result<usize, Error> {
    let file = File::open(path).map_err(|source| Error::Open {
        path: path.to_path_buf(),
        source,
    })?;
    count_impl(&mut BufReader::new(file), Some(path), metric)
}

fn count_impl<R: BufRead>(
    reader: &mut R,
    path: Option<&Path>,
    metric: Metric,
) -> Result<usize, Error> {
    let mut buffer = String::new();
    let mut total = 0usize;

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            // End of input.
            Ok(0) => break,
            // Counting per line is exact: neither words nor letters can span
            // a line break, because a line break is itself a separator.
            Ok(_) => total += metric.count_line(&buffer),
            Err(source) => {
                return Err(Error::Read {
                    path: path.map(Path::to_path_buf),
                    source,
                });
            }
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn words(text: &str) -> usize {
        count_words_in_text(text)
    }

    fn letters(text: &str) -> usize {
        count_letters_in_text(text)
    }

    #[test]
    fn empty_input_has_no_words() {
        assert_eq!(words(""), 0);
    }

    #[test]
    fn whitespace_only_input_has_no_words() {
        assert_eq!(words("   \n\t\r\n  "), 0);
    }

    #[test]
    fn consecutive_spaces_count_as_one_separator() {
        assert_eq!(words("hi my name is   arokoyu"), 5);
    }

    #[test]
    fn words_are_counted_across_lines_and_crlf() {
        assert_eq!(words("one two\r\nthree\r\n\r\nfour  "), 4);
    }

    #[test]
    fn unicode_text_is_counted_per_word() {
        assert_eq!(words("héllo wörld\nnaïve\tcafé"), 4);
    }

    #[test]
    fn letters_ignore_digits_punctuation_and_whitespace() {
        assert_eq!(letters(""), 0);
        assert_eq!(letters("   \n\t "), 0);
        assert_eq!(letters("abc 123 !? \n\t"), 3);
    }

    #[test]
    fn letters_include_unicode_letters() {
        assert_eq!(letters("héllo, wörld! 42"), 10);
    }

    #[test]
    fn letters_are_counted_across_lines_and_crlf() {
        assert_eq!(letters("ab\r\n\r\ncd ef"), 6);
    }

    #[test]
    fn counts_letters_in_the_shipped_fixture_file() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hello.txt");
        assert_eq!(count_letters_in_file(&fixture).unwrap(), 53);
    }

    #[test]
    fn streaming_letter_counter_matches_in_memory_counter() {
        let sample = "Alpha  BETA\r\ngamma 42!\n";
        let mut reader = Cursor::new(sample);
        assert_eq!(count_letters(&mut reader).unwrap(), letters(sample));
    }

    #[test]
    fn streaming_reader_matches_in_memory_counter() {
        let sample = "alpha  beta\r\ngamma\n\ndelta epsilon\n";
        let mut reader = Cursor::new(sample);
        assert_eq!(count_words(&mut reader).unwrap(), words(sample));
    }

    #[test]
    fn counts_the_shipped_fixture_file() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hello.txt");
        assert_eq!(count_words_in_file(&fixture).unwrap(), 14);
    }

    #[test]
    fn missing_file_reports_the_path_it_could_not_open() {
        let missing = Path::new("/definitely/not/a/real/file.txt");
        match count_words_in_file(missing) {
            Err(Error::Open { path, .. }) => assert_eq!(path, missing),
            other => panic!("expected Error::Open, got {other:?}"),
        }
    }

    #[test]
    fn non_utf8_input_reports_a_read_error_with_the_path() {
        let path =
            std::env::temp_dir().join(format!("word_counter_bad_utf8_{}", std::process::id()));
        std::fs::write(&path, [b'h', b'i', 0xFF, 0xFE]).unwrap();

        let result = count_words_in_file(&path);
        let _ = std::fs::remove_file(&path);

        match result {
            Err(Error::Read {
                path: Some(read_path),
                source,
            }) => {
                assert_eq!(read_path, path);
                assert_eq!(source.kind(), io::ErrorKind::InvalidData);
            }
            other => panic!("expected Error::Read with a path, got {other:?}"),
        }
    }
}
