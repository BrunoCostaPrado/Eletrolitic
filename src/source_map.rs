use crate::token::Span;
use serde::{Deserialize, Serialize};

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
  source_name: String,
  source_content: Option<String>,
  line_starts: Vec<usize>,
  mappings: Vec<Mapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mapping {
  gen_line: usize,
  gen_col: usize,
  orig_line: usize,
  orig_col: usize,
}

impl SourceMap {
  #[must_use]
  pub fn new(source_text: &str, source_name: &str, source_content: Option<&str>) -> Self {
    let line_starts: Vec<usize> =
      std::iter::once(0).chain(source_text.match_indices('\n').map(|(i, _)| i + 1)).collect();
    Self {
      source_name: source_name.to_string(),
      source_content: source_content.map(|s| s.to_string()),
      line_starts,
      mappings: Vec::new(),
    }
  }

  /// Convert byte offset to (line, col) — both 0-based.
  #[must_use]
  pub fn byte_to_line_col(&self, offset: usize) -> (usize, usize) {
    match self.line_starts.binary_search(&offset) {
      Ok(line) => (line, 0),
      Err(line) => (line - 1, offset - self.line_starts[line - 1]),
    }
  }

  /// Record a mapping from generated position to source span.
  /// Generated position is passed directly (line/col 0-based).
  pub fn add_mapping(&mut self, gen_line: usize, gen_col: usize, span: Span) {
    let (orig_line, orig_col) = self.byte_to_line_col(span.start);
    self.mappings.push(Mapping { gen_line, gen_col, orig_line, orig_col });
  }

  /// Serialize to source map JSON (v3 spec).
  #[must_use]
  pub fn to_json(&self, generated_file: &str) -> String {
    // ponytail: skip clone+sort — add_mapping called in gen_line/gen_col order
    let encoded = Self::encode_mappings(&self.mappings);
    let sources_content = match &self.source_content {
      Some(c) => format!(",\"sourcesContent\":[\"{}\"]", json_escape(c)),
      None => String::new(),
    };
    format!(
      r#"{{"version":3,"file":"{generated_file}","sources":["{}"]{sources_content},"names":[],"mappings":"{encoded}"}}"#,
      self.source_name
    )
  }

  fn encode_mappings(mappings: &[Mapping]) -> String {
    let mut result = String::new();
    let mut prev_gen_line: usize = 0;
    let mut prev_gen_col: usize = 0;
    let mut prev_orig_line: usize = 0;
    let mut prev_orig_col: usize = 0;

    for m in mappings {
      // Semicolons for new generated lines
      while prev_gen_line < m.gen_line {
        result.push(';');
        prev_gen_line += 1;
        prev_gen_col = 0;
      }
      if !result.is_empty() && prev_gen_line == m.gen_line {
        result.push(',');
      }

      // VLQ: gen_col delta, source_index(0), orig_line delta, orig_col delta
      result.push_str(&vlq_encode(m.gen_col as i64 - prev_gen_col as i64));
      result.push_str(&vlq_encode(0));
      result.push_str(&vlq_encode(m.orig_line as i64 - prev_orig_line as i64));
      result.push_str(&vlq_encode(m.orig_col as i64 - prev_orig_col as i64));

      prev_gen_col = m.gen_col;
      prev_orig_line = m.orig_line;
      prev_orig_col = m.orig_col;
    }

    result
  }
}

fn vlq_encode(mut value: i64) -> String {
  let mut result = String::new();
  let sign = i64::from(value < 0);
  value = value.abs() << 1 | sign;

  loop {
    let mut digit = (value & 0x1F) as u8;
    value >>= 5;
    if value > 0 {
      digit |= 0x20;
    }
    result.push(BASE64[digit as usize] as char);
    if value == 0 {
      break;
    }
  }

  result
}

fn json_escape(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      _ => out.push(c),
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn vlq_zero() {
    assert_eq!(vlq_encode(0), "A");
  }

  #[test]
  fn vlq_one() {
    assert_eq!(vlq_encode(1), "C");
  }

  #[test]
  fn vlq_minus_one() {
    assert_eq!(vlq_encode(-1), "D");
  }

  #[test]
  fn vlq_fifteen() {
    assert_eq!(vlq_encode(15), "e");
  }

  #[test]
  fn vlq_sixteen() {
    assert_eq!(vlq_encode(16), "gB");
  }

  #[test]
  fn byte_to_line_col_simple() {
    let sm = SourceMap::new("hello\nworld\n", "test.ts", None);
    assert_eq!(sm.byte_to_line_col(0), (0, 0));
    assert_eq!(sm.byte_to_line_col(4), (0, 4));
    assert_eq!(sm.byte_to_line_col(5), (0, 5)); // \n
    assert_eq!(sm.byte_to_line_col(6), (1, 0));
    assert_eq!(sm.byte_to_line_col(10), (1, 4));
  }

  #[test]
  fn single_mapping_json() {
    let mut sm = SourceMap::new("let x = 1;\n", "input.ts", None);
    sm.add_mapping(0, 0, Span::new(0, 11));
    let json = sm.to_json("output.js");
    assert!(json.contains("\"version\":3"));
    assert!(json.contains("\"file\":\"output.js\""));
    assert!(json.contains("\"sources\":[\"input.ts\"]"));
    assert!(json.contains("\"mappings\":\""));
  }

  #[test]
  fn multiple_mappings_encode() {
    let mut sm = SourceMap::new("let a = 1;\nlet b = 2;\n", "input.ts", None);
    sm.add_mapping(0, 0, Span::new(0, 11));
    sm.add_mapping(1, 0, Span::new(11, 22));
    let json = sm.to_json("out.js");
    // Should have a semicolon separating line 0 and line 1
    let mappings_start = json.find("\"mappings\":\"").unwrap() + 12;
    let mappings_end = json[mappings_start..].find('"').unwrap() + mappings_start;
    let mappings = &json[mappings_start..mappings_end];
    assert!(mappings.contains(';'), "expected semicolon in mappings: {mappings}");
  }

  #[test]
  fn source_map_is_valid_json() {
    let mut sm = SourceMap::new("x;\n", "a.ts", None);
    sm.add_mapping(0, 0, Span::new(0, 2));
    let json = sm.to_json("a.js");
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
    assert!(json.contains("\"version\":3"));
    assert!(json.contains("\"mappings\":\""));
    assert!(json.contains("\"sources\":[\"a.ts\"]"));
    assert!(json.contains("\"file\":\"a.js\""));
  }

  #[test]
  fn json_escape_basic() {
    assert_eq!(json_escape("hello"), "hello");
    assert_eq!(json_escape("a\"b"), "a\\\"b");
    assert_eq!(json_escape("a\\b"), "a\\\\b");
    assert_eq!(json_escape("a\nb"), "a\\nb");
  }

  #[test]
  fn source_map_with_content() {
    let mut sm = SourceMap::new("let x = 1;\n", "input.ts", Some("let x = 1;\n"));
    sm.add_mapping(0, 0, Span::new(0, 11));
    let json = sm.to_json("output.js");
    assert!(
      json.contains("\"sourcesContent\":[\"let x = 1;\\n\"]"),
      "missing sourcesContent: {json}"
    );
    assert!(json.contains("\"sources\":[\"input.ts\"]"));
    assert!(json.contains("\"file\":\"output.js\""));
  }

  #[test]
  fn source_map_without_content_omits_field() {
    let mut sm = SourceMap::new("let x = 1;\n", "input.ts", None);
    sm.add_mapping(0, 0, Span::new(0, 11));
    let json = sm.to_json("output.js");
    assert!(!json.contains("sourcesContent"), "should omit sourcesContent: {json}");
  }
}
