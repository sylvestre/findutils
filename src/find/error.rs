// This file is part of the uutils findutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Rich diagnostics for `find` expression parse errors.
//!
//! The parser knows exactly which command-line argument it choked on, but that
//! information used to be dropped on the floor: every error was a `String`
//! boxed into a `Box<dyn Error>`. [`ParseError`] keeps the argument index
//! alongside the message so that, when the user opts in with `UU_DIAG`, the
//! offending argument can be underlined in the reconstructed command line.
//!
//! The `Display` implementation yields the bare message, unchanged from what
//! `find` has always printed, so the default output stays byte-for-byte
//! GNU-compatible.

use std::env;
use std::error::Error;
use std::io::Write;
use std::ops::Range;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};

/// Name shown in the report header, standing in for a file name.
const SOURCE_ID: &str = "command line";

/// A command-line expression error that can point at the argument that caused it.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    message: String,
    /// Index of the offending argument, relative to the argument slice this
    /// error was created in. [`ParseError::shift`] moves it along at each
    /// parsing-layer boundary until it indexes the process's full argv.
    arg_index: Option<usize>,
    label: Option<String>,
    help: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            arg_index: None,
            label: None,
            help: None,
        }
    }

    /// Records which argument the error refers to.
    pub fn at(mut self, arg_index: usize) -> Self {
        self.arg_index = Some(arg_index);
        self
    }

    /// Sets the text shown under the underlined argument.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a suggestion shown below the report.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn arg_index(&self) -> Option<usize> {
        self.arg_index
    }

    /// Moves a `ParseError`'s argument index by `by` positions.
    ///
    /// The parser works on sub-slices of argv, so an index is only meaningful
    /// relative to the slice it was produced in. Each caller that narrowed the
    /// slice shifts the index back by the amount it skipped. Errors of any
    /// other type pass through untouched.
    pub fn shift(err: Box<dyn Error>, by: usize) -> Box<dyn Error> {
        match err.downcast::<Self>() {
            Ok(mut parse_error) => {
                if let Some(index) = parse_error.arg_index {
                    parse_error.arg_index = Some(index + by);
                }
                parse_error
            }
            Err(other) => other,
        }
    }

    /// Writes an underlined report of this error over the reconstructed
    /// command line, and reports whether anything was written.
    ///
    /// The report leads with the same `find: <message>` line the plain output
    /// uses, so enabling diagnostics adds context rather than replacing it.
    /// Errors that don't name an argument can't be drawn and render nothing.
    pub fn render(&self, argv: &[&str], colored: bool, out: &mut impl Write) -> bool {
        let Some(arg_index) = self.arg_index else {
            return false;
        };
        let (command_line, spans) = render_command_line(argv);
        let Some(span) = spans.get(arg_index) else {
            return false;
        };

        let mut report = Report::build(ReportKind::Error, (SOURCE_ID, span.clone()))
            .with_config(
                Config::default()
                    .with_color(colored)
                    .with_index_type(IndexType::Byte),
            )
            .with_label(
                Label::new((SOURCE_ID, span.clone()))
                    .with_message(self.label.as_deref().unwrap_or("here"))
                    .with_color(Color::Red),
            );
        if let Some(help) = &self.help {
            report = report.with_help(help);
        }

        let mut buffer = Vec::new();
        if report
            .finish()
            .write((SOURCE_ID, Source::from(&command_line)), &mut buffer)
            .is_err()
        {
            return false;
        }

        // Ariadne opens every report with a one-line header of its own. Drop it
        // in favour of find's usual wording, so the first line of the
        // diagnostic is exactly what the plain output would have been.
        let body_start = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(0, |newline| newline + 1);
        writeln!(out, "find: {}", self.message).is_ok()
            && out.write_all(&buffer[body_start..]).is_ok()
    }
}

/// True when the user asked for rich diagnostics via `UU_DIAG`.
pub fn diagnostics_enabled() -> bool {
    env::var_os("UU_DIAG").is_some_and(|value| !value.is_empty() && value != "0")
}

/// Rebuilds a displayable command line from `argv`, returning it along with the
/// byte range each argument occupies in it.
///
/// Arguments are quoted the way a shell would need them to be, so the rendered
/// line is something the user can recognise (and re-run).
fn render_command_line(argv: &[&str]) -> (String, Vec<Range<usize>>) {
    let mut line = String::new();
    let mut spans = Vec::with_capacity(argv.len());

    for arg in argv {
        if !line.is_empty() {
            line.push(' ');
        }
        let start = line.len();
        push_quoted(&mut line, arg);
        spans.push(start..line.len());
    }

    (line, spans)
}

/// Appends `arg` to `line`, single-quoting it if a shell would need it.
fn push_quoted(line: &mut String, arg: &str) {
    const SAFE_PUNCTUATION: &str = "_./:=@,+-";

    let needs_quoting = arg.is_empty()
        || !arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || SAFE_PUNCTUATION.contains(c));

    if !needs_quoting {
        line.push_str(arg);
        return;
    }

    line.push('\'');
    for c in arg.chars() {
        if c == '\'' {
            // A single quote can't appear inside single quotes: close, escape,
            // reopen.
            line.push_str("'\\''");
        } else {
            line.push(c);
        }
    }
    line.push('\'');
}

/// Returns the entry of `candidates` closest to `input`, if one is close enough
/// to be a plausible typo.
pub fn closest_match<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    // Allow one edit for short predicates and more for longer ones, but never
    // so many that unrelated names start matching. Two edits is the useful
    // minimum for anything but the shortest names, since a swapped pair of
    // letters already costs that much.
    let max_distance = match input.chars().count() {
        0..=3 => 1,
        4..=7 => 2,
        _ => 3,
    };

    candidates
        .iter()
        .map(|candidate| (edit_distance(input, candidate), *candidate))
        .filter(|(distance, _)| *distance <= max_distance)
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance between two strings, counting characters rather than
/// bytes.
fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    // Distances from the empty prefix of `left` to each prefix of `right`.
    let mut previous_row: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current_row = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current_row[0] = left_index + 1;
        for (right_index, &right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != right_char);
            current_row[right_index + 1] = (current_row[right_index] + 1)
                .min(previous_row[right_index + 1] + 1)
                .min(previous_row[right_index] + substitution_cost);
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }

    previous_row[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(error: &ParseError, argv: &[&str]) -> String {
        let mut buffer = Vec::new();
        let rendered = error.render(argv, false, &mut buffer);
        let output = String::from_utf8(buffer).unwrap();
        assert_eq!(rendered, !output.is_empty());
        output
    }

    #[test]
    fn display_is_the_bare_message() {
        let error = ParseError::new("unknown predicate `-zap'")
            .at(3)
            .with_label("not a known predicate")
            .with_help("did you mean `-zip'?");
        assert_eq!(error.to_string(), "unknown predicate `-zap'");
    }

    #[test]
    fn render_underlines_the_offending_argument() {
        let error = ParseError::new("unknown predicate `-zap'")
            .at(3)
            .with_label("not a known predicate");
        let output = render_to_string(&error, &["find", "/srv", "-type", "-zap"]);

        assert!(output.starts_with("find: unknown predicate `-zap'\n"));
        assert!(output.contains("not a known predicate"));

        // The underline must start under the first character of `-zap` in the
        // rendered command line, and be as wide as it.
        let source_line = output
            .lines()
            .find(|line| line.contains("find /srv -type -zap"))
            .expect("command line not shown");
        let underline = output
            .lines()
            .find(|line| line.contains('┬'))
            .expect("no underline drawn");
        assert_eq!(
            underline.chars().position(|c| c == '─'),
            source_line
                .find("-zap")
                .map(|index| source_line[..index].chars().count())
        );
        assert_eq!(underline.chars().filter(|c| *c == '─').count() + 1, 4);
    }

    #[test]
    fn render_includes_the_help_text() {
        let error = ParseError::new("unknown predicate `-nmae'")
            .at(1)
            .with_help("did you mean `-name'?");
        let output = render_to_string(&error, &["find", "-nmae"]);
        assert!(output.contains("did you mean `-name'?"));
    }

    #[test]
    fn render_without_an_index_writes_nothing() {
        let error = ParseError::new("something went wrong");
        assert_eq!(render_to_string(&error, &["find", "/srv"]), "");
    }

    #[test]
    fn render_with_an_out_of_range_index_writes_nothing() {
        let error = ParseError::new("something went wrong").at(9);
        assert_eq!(render_to_string(&error, &["find", "/srv"]), "");
    }

    #[test]
    fn quoted_arguments_keep_their_span() {
        let (line, spans) = render_command_line(&["find", "/var log", "-name", "*.gz"]);
        assert_eq!(line, "find '/var log' -name '*.gz'");
        assert_eq!(&line[spans[1].clone()], "'/var log'");
        assert_eq!(&line[spans[3].clone()], "'*.gz'");
    }

    #[test]
    fn quoting_handles_embedded_quotes_and_empty_arguments() {
        let (line, spans) = render_command_line(&["find", "it's", ""]);
        assert_eq!(line, r"find 'it'\''s' ''");
        assert_eq!(&line[spans[2].clone()], "''");
    }

    #[test]
    fn multibyte_arguments_get_byte_accurate_spans() {
        let (line, spans) = render_command_line(&["find", "café", "-print"]);
        assert_eq!(&line[spans[2].clone()], "-print");
    }

    #[test]
    fn shift_moves_the_index_and_composes() {
        let error: Box<dyn Error> = Box::new(ParseError::new("oops").at(2));
        let error = ParseError::shift(ParseError::shift(error, 3), 1);
        let shifted = error.downcast_ref::<ParseError>().unwrap();
        assert_eq!(shifted.arg_index(), Some(6));
    }

    #[test]
    fn shift_leaves_indexless_and_foreign_errors_alone() {
        let indexless: Box<dyn Error> = Box::new(ParseError::new("oops"));
        let indexless = ParseError::shift(indexless, 4);
        assert_eq!(
            indexless.downcast_ref::<ParseError>().unwrap().arg_index(),
            None
        );

        let foreign: Box<dyn Error> = From::from("plain message");
        let foreign = ParseError::shift(foreign, 4);
        assert_eq!(foreign.to_string(), "plain message");
        assert!(foreign.downcast_ref::<ParseError>().is_none());
    }

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("-name", "-name"), 0);
        assert_eq!(edit_distance("-nmae", "-name"), 2);
        assert_eq!(edit_distance("-nam", "-name"), 1);
        assert_eq!(edit_distance("", "-name"), 5);
        assert_eq!(edit_distance("-name", ""), 5);
    }

    #[test]
    fn closest_match_finds_plausible_typos() {
        let candidates = ["-name", "-newer", "-nogroup", "-print"];
        assert_eq!(closest_match("-nmae", &candidates), Some("-name"));
        assert_eq!(closest_match("-printt", &candidates), Some("-print"));
        assert_eq!(closest_match("-nogruop", &candidates), Some("-nogroup"));
    }

    #[test]
    fn closest_match_rejects_distant_input() {
        let candidates = ["-name", "-newer", "-print"];
        assert_eq!(closest_match("-zzzzzzzzzz", &candidates), None);
        assert_eq!(closest_match("-xyz", &candidates), None);
    }
}
