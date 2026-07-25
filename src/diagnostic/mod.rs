use std::fmt::Write;

use crate::token::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
  Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
  pub severity: Severity,
  pub code: String,
  pub message: String,
  pub span: Span,
}

impl Diagnostic {
  pub fn error(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
    Self { severity: Severity::Error, code: code.into(), message: message.into(), span }
  }
}

#[derive(Clone)]
pub struct SourceFile {
  pub path: String,
  pub source: String,
  line_starts: Vec<usize>,
}

impl SourceFile {
  pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
    let source: String = source.into();
    let line_starts: Vec<usize> =
      std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1)).collect();
    Self { path: path.into(), source, line_starts }
  }

  /// Returns (1-indexed line, 1-indexed column) for a byte offset.
  #[must_use]
  pub fn line_col(&self, offset: usize) -> (usize, usize) {
    match self.line_starts.binary_search(&offset) {
      Ok(line) => (line + 1, 1),
      Err(line) => (line, offset - self.line_starts[line - 1] + 1),
    }
  }

  /// Returns the byte range of a 1-indexed line (including newline).
  #[must_use]
  pub fn line_range(&self, line: usize) -> std::ops::Range<usize> {
    let start = self.line_starts[line - 1];
    let end =
      if line < self.line_starts.len() { self.line_starts[line] } else { self.source.len() };
    start..end
  }

  /// Returns the source text of a 1-indexed line, trimmed of trailing newline.
  #[must_use]
  pub fn line_text(&self, line: usize) -> &str {
    let range = self.line_range(line);
    self.source[range].trim_end_matches(['\r', '\n'])
  }

  /// Format an error with source context and caret pointing to the error location.
  #[must_use]
  pub fn format_error(&self, code: &str, message: &str, span: Span) -> String {
    let (start_line, start_col) = self.line_col(span.start);
    let (end_line, end_col) = self.line_col(span.end);
    let text = self.line_text(start_line);

    // Header: error code + message
    let mut out = format!("error[{code}]: {message}");

    // Location: --> file:line:col
    let _ = write!(out, "\n  --> {}:{start_line}:{start_col}", self.path);

    // Separator line
    let _ = write!(out, "\n   |");

    // Source line with line number
    let gutter = format!(" {} | ", start_line);
    let _ = write!(out, "\n{gutter}{text}");

    // Caret line — underline the span on the error line
    let caret_col = start_col.saturating_sub(1); // 0-based offset into text
    let caret_len =
      if end_line == start_line && end_col > start_col { (end_col - start_col).max(1) } else { 1 };
    let prefix = " ".repeat(gutter.len());
    let carets = "^".repeat(caret_len);
    let _ = write!(out, "\n{prefix}{carets}");

    out
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn source_file_line_col_single_line() {
    let sf = SourceFile::new("test.ts", "hello");
    assert_eq!(sf.line_col(0), (1, 1));
    assert_eq!(sf.line_col(4), (1, 5));
  }

  #[test]
  fn source_file_line_col_multi_line() {
    let sf = SourceFile::new("test.ts", "a\nb\nc");
    assert_eq!(sf.line_col(0), (1, 1));
    assert_eq!(sf.line_col(2), (2, 1));
    assert_eq!(sf.line_col(3), (2, 2));
    assert_eq!(sf.line_col(4), (3, 1));
  }

  #[test]
  fn source_file_line_range() {
    let sf = SourceFile::new("test.ts", "abc\nde\nf");
    assert_eq!(sf.line_range(1), 0..4);
    assert_eq!(sf.line_range(2), 4..7);
    assert_eq!(sf.line_range(3), 7..8);
  }
}
