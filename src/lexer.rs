use crate::diagnostic::Diagnostic;
use crate::token::{Span, Token, TokenKind};

pub struct Lexer<'a> {
  source: &'a str,
  start: usize,
  current: usize,
  tokens: Vec<Token>,
  diagnostics: Vec<Diagnostic>,
  in_template: bool,
  in_template_expr: bool,
  template_expr_depth: usize,
  template_has_head: bool,
}

impl<'a> Lexer<'a> {
  #[must_use]
  pub fn new(source: &'a str) -> Self {
    Self {
      source,
      start: 0,
      current: 0,
      tokens: Vec::new(),
      diagnostics: Vec::new(),
      in_template: false,
      in_template_expr: false,
      template_expr_depth: 0,
      template_has_head: false,
    }
  }

  pub fn tokenize(&mut self) -> &[Token] {
    while !self.is_at_end() {
      self.start = self.current;
      self.scan_token();
    }
    self.tokens.push(Token::new(TokenKind::Eof, Span::new(self.current, self.current)));
    &self.tokens
  }

  #[must_use]
  pub fn into_diagnostics(self) -> Vec<Diagnostic> {
    self.diagnostics
  }

  fn is_at_end(&self) -> bool {
    self.current >= self.source.len()
  }

  fn advance(&mut self) -> char {
    // ponytail: ASCII fast path — avoids UTF-8 decode for 99%+ of TS source
    let b = self.source.as_bytes()[self.current];
    if b < 128 {
      self.current += 1;
      b as char
    } else {
      let c = self.source[self.current..].chars().next().unwrap();
      self.current += c.len_utf8();
      c
    }
  }

  fn peek(&self) -> char {
    if self.current >= self.source.len() {
      return '\0';
    }
    let b = self.source.as_bytes()[self.current];
    if b < 128 { b as char } else { self.source[self.current..].chars().next().unwrap_or('\0') }
  }

  fn peek_next(&self) -> char {
    if self.current >= self.source.len() {
      return '\0';
    }
    let first = self.source.as_bytes()[self.current];
    let step = if first < 128 {
      1
    } else {
      self.source[self.current..].chars().next().unwrap_or('\0').len_utf8()
    };
    let pos = self.current + step;
    if pos >= self.source.len() {
      return '\0';
    }
    let b = self.source.as_bytes()[pos];
    if b < 128 { b as char } else { self.source[pos..].chars().next().unwrap_or('\0') }
  }

  fn add_token(&mut self, kind: TokenKind) {
    let span = Span::new(self.start, self.current);
    self.tokens.push(Token::new(kind, span));
  }

  fn scan_token(&mut self) {
    let c = self.advance();
    match c {
      '(' => self.add_token(TokenKind::OpenParen),
      ')' => self.add_token(TokenKind::CloseParen),
      '[' => self.add_token(TokenKind::OpenBracket),
      ']' => self.add_token(TokenKind::CloseBracket),
      '{' => {
        self.add_token(TokenKind::OpenBrace);
        if self.in_template_expr {
          self.template_expr_depth += 1;
        }
      }
      '}' => {
        if self.in_template_expr {
          if self.template_expr_depth > 0 {
            self.template_expr_depth -= 1;
            self.add_token(TokenKind::CloseBrace);
          } else {
            self.in_template_expr = false;
            self.scan_template_text();
          }
        } else {
          self.add_token(TokenKind::CloseBrace);
        }
      }
      ',' => self.add_token(TokenKind::Comma),
      ';' => self.add_token(TokenKind::Semicolon),
      ':' => self.add_token(TokenKind::Colon),
      '.' => {
        if self.peek() == '.' && self.peek_next() == '.' {
          self.advance();
          self.advance();
          self.add_token(TokenKind::DotDotDot);
        } else {
          self.add_token(TokenKind::Dot);
        }
      }
      '?' => {
        if self.peek() == '?' {
          self.advance();
          if self.peek() == '=' {
            self.advance();
            self.add_token(TokenKind::QuestionQuestionEq);
          } else {
            self.add_token(TokenKind::QuestionQuestion);
          }
        } else if self.peek() == '.' {
          self.advance();
          self.add_token(TokenKind::QuestionDot);
        } else {
          self.add_token(TokenKind::Question);
        }
      }
      '+' => {
        if self.peek() == '+' {
          self.advance();
          self.add_token(TokenKind::PlusPlus);
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::PlusEq);
        } else {
          self.add_token(TokenKind::Plus);
        }
      }
      '-' => {
        if self.peek() == '-' {
          self.advance();
          self.add_token(TokenKind::MinusMinus);
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::MinusEq);
        } else if self.peek() == '>' {
          self.advance();
          self.add_token(TokenKind::Arrow);
        } else {
          self.add_token(TokenKind::Minus);
        }
      }
      '*' => {
        if self.peek() == '*' {
          self.advance();
          self.add_token(TokenKind::StarStar);
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::StarEq);
        } else {
          self.add_token(TokenKind::Star);
        }
      }
      '/' => {
        if self.peek() == '/' {
          // ponytail: byte-level line comment scan
          let bytes = self.source.as_bytes();
          while self.current < bytes.len() && bytes[self.current] != b'\n' {
            self.current += 1;
          }
        } else if self.peek() == '*' {
          self.advance();
          let mut depth = 1i32;
          // ponytail: byte-level block comment scan
          let bytes = self.source.as_bytes();
          while self.current < bytes.len() {
            let b = bytes[self.current];
            if b == b'*' && self.current + 1 < bytes.len() && bytes[self.current + 1] == b'/' {
              self.current += 2;
              depth -= 1;
              if depth == 0 {
                break;
              }
            } else if b == b'/' && self.current + 1 < bytes.len() && bytes[self.current + 1] == b'*'
            {
              self.current += 2;
              depth += 1;
            } else {
              self.current += 1;
            }
          }
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::SlashEq);
        } else {
          self.add_token(TokenKind::Slash);
        }
      }
      '%' => {
        if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::PercentEq);
        } else {
          self.add_token(TokenKind::Percent);
        }
      }
      '=' => {
        if self.peek() == '=' {
          self.advance();
          if self.peek() == '=' {
            self.advance();
            self.add_token(TokenKind::EqEqEq);
          } else {
            self.add_token(TokenKind::EqEq);
          }
        } else if self.peek() == '>' {
          self.advance();
          self.add_token(TokenKind::Arrow);
        } else {
          self.add_token(TokenKind::Eq);
        }
      }
      '!' => {
        if self.peek() == '=' {
          self.advance();
          if self.peek() == '=' {
            self.advance();
            self.add_token(TokenKind::NotEqEq);
          } else {
            self.add_token(TokenKind::NotEq);
          }
        } else {
          self.add_token(TokenKind::Bang);
        }
      }
      '<' => {
        if self.peek() == '<' {
          self.advance();
          self.add_token(TokenKind::LtLt);
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::LtEq);
        } else {
          self.add_token(TokenKind::Lt);
        }
      }
      '>' => {
        if self.peek() == '>' {
          self.advance();
          if self.peek() == '>' {
            self.advance();
            self.add_token(TokenKind::GtGtGt);
          } else {
            self.add_token(TokenKind::GtGt);
          }
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::GtEq);
        } else {
          self.add_token(TokenKind::Gt);
        }
      }
      '&' => {
        if self.peek() == '&' {
          self.advance();
          if self.peek() == '=' {
            self.advance();
            self.add_token(TokenKind::AmpAmpEq);
          } else {
            self.add_token(TokenKind::AmpAmp);
          }
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::AmpEq);
        } else {
          self.add_token(TokenKind::Amp);
        }
      }
      '|' => {
        if self.peek() == '|' {
          self.advance();
          if self.peek() == '=' {
            self.advance();
            self.add_token(TokenKind::PipePipeEq);
          } else {
            self.add_token(TokenKind::PipePipe);
          }
        } else if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::PipeEq);
        } else {
          self.add_token(TokenKind::Pipe);
        }
      }
      '^' => {
        if self.peek() == '=' {
          self.advance();
          self.add_token(TokenKind::CaretEq);
        } else {
          self.add_token(TokenKind::Caret);
        }
      }
      '~' => self.add_token(TokenKind::Tilde),
      '"' | '\'' => self.string(c),
      '`' => {
        if self.in_template {
          // Nested template not supported — close current
          self.in_template = false;
          self.in_template_expr = false;
          self.add_token(TokenKind::TemplateTail(String::new()));
        } else {
          self.in_template = true;
          self.template_has_head = false;
          self.scan_template_text();
        }
      }
      c if c.is_ascii_digit() => self.number(),
      c if c.is_ascii_alphabetic() || c == '_' || c == '$' => self.identifier(),
      '@' => self.add_token(TokenKind::At),
      c if c.is_whitespace() => {}
      _ => {
        let span = Span::new(self.start, self.current);
        self.diagnostics.push(Diagnostic::error("E004", format!("Unknown character '{c}'"), span));
        self.add_token(TokenKind::Unknown(c));
      }
    }
  }

  fn string(&mut self, quote: char) {
    // ponytail: byte-level scan for quote char (always ASCII: " or ')
    let q = quote as u8;
    let bytes = self.source.as_bytes();
    while self.current < bytes.len() && bytes[self.current] != q {
      if bytes[self.current] == b'\\' {
        self.current += 1; // skip escape char
        if self.current < bytes.len() {
          self.current += 1; // skip escaped char
        }
      } else {
        self.current += 1;
      }
    }
    if self.is_at_end() {
      let span = Span::new(self.start, self.current);
      self.diagnostics.push(Diagnostic::error("E003", "Unterminated string literal", span));
      let value = self.source[self.start + 1..self.current].to_string();
      self.add_token(TokenKind::String(value));
    } else {
      self.advance();
      let value = self.source[self.start + 1..self.current - 1].to_string();
      self.add_token(TokenKind::String(value));
    }
  }

  fn number(&mut self) {
    // ponytail: byte-level scan, avoids UTF-8 decode per digit
    let bytes = self.source.as_bytes();
    while self.current < bytes.len() && bytes[self.current].is_ascii_digit() {
      self.current += 1;
    }
    if self.current < bytes.len() && bytes[self.current] == b'.' {
      let next = self.current + 1;
      if next < bytes.len() && bytes[next].is_ascii_digit() {
        self.current = next + 1;
        while self.current < bytes.len() && bytes[self.current].is_ascii_digit() {
          self.current += 1;
        }
      }
    }
    let value: f64 = self.source[self.start..self.current].parse().unwrap_or(0.0);
    self.add_token(TokenKind::Number(value));
  }

  fn identifier(&mut self) {
    // ponytail: byte-level scan, avoids UTF-8 decode per char
    let bytes = self.source.as_bytes();
    while self.current < bytes.len() {
      let b = bytes[self.current];
      if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' {
        self.current += 1;
      } else {
        break;
      }
    }
    let word = &self.source[self.start..self.current];
    let kind = match word {
      "true" => TokenKind::True,
      "false" => TokenKind::False,
      "null" => TokenKind::Null,
      "undefined" => TokenKind::Undefined,
      "let" => TokenKind::Let,
      "const" => TokenKind::Const,
      "var" => TokenKind::Var,
      "function" => TokenKind::Function,
      "return" => TokenKind::Return,
      "if" => TokenKind::If,
      "else" => TokenKind::Else,
      "for" => TokenKind::For,
      "while" => TokenKind::While,
      "typeof" => TokenKind::TypeOf,
      "keyof" => TokenKind::KeyOf,
      "void" => TokenKind::Void,
      "delete" => TokenKind::Delete,
      "class" => TokenKind::Class,
      "import" => TokenKind::Import,
      "from" => TokenKind::From,
      "export" => TokenKind::Export,
      "as" => TokenKind::As,
      "default" => TokenKind::Default,
      "type" => TokenKind::Type,
      "switch" => TokenKind::Switch,
      "case" => TokenKind::Case,
      "throw" => TokenKind::Throw,
      "try" => TokenKind::Try,
      "catch" => TokenKind::Catch,
      "finally" => TokenKind::Finally,
      "of" => TokenKind::Of,
      "in" => TokenKind::In,
      "break" => TokenKind::Break,
      "continue" => TokenKind::Continue,
      "new" => TokenKind::New,
      "do" => TokenKind::Do,
      "this" => TokenKind::This,
      "super" => TokenKind::Super,
      "extends" => TokenKind::Extends,
      "static" => TokenKind::Static,
      "async" => TokenKind::Async,
      "await" => TokenKind::Await,
      "enum" => TokenKind::Enum,
      "interface" => TokenKind::Interface,
      "instanceof" => TokenKind::Instanceof,
      "public" => TokenKind::Public,
      "private" => TokenKind::Private,
      "protected" => TokenKind::Protected,
      "satisfies" => TokenKind::Satisfies,
      "meta" => TokenKind::Meta,
      "declare" => TokenKind::Declare,
      "module" => TokenKind::Module,
      "readonly" => TokenKind::Readonly,
      _ => TokenKind::Identifier(word.to_string()),
    };
    self.add_token(kind);
  }

  fn scan_template_text(&mut self) {
    let mut text = String::new();
    loop {
      if self.is_at_end() {
        self.diagnostics.push(Diagnostic::error(
          "E003",
          "Unterminated template literal",
          Span::new(self.start, self.current),
        ));
        self.in_template = false;
        self.add_token(TokenKind::TemplateTail(text));
        return;
      }
      let c = self.peek();
      if c == '`' {
        self.advance();
        self.in_template = false;
        if self.template_has_head {
          self.add_token(TokenKind::TemplateTail(text));
        } else {
          self.add_token(TokenKind::NoSubstitutionTemplate(text));
        }
        return;
      }
      if c == '$' && self.peek_next() == '{' {
        self.advance(); // $
        self.advance(); // {
        if self.template_has_head {
          self.add_token(TokenKind::TemplateMiddle(text));
        } else {
          self.add_token(TokenKind::TemplateHead(text));
        }
        self.template_has_head = true;
        self.in_template_expr = true;
        self.template_expr_depth = 0;
        return;
      }
      if c == '\\' {
        self.advance();
        if !self.is_at_end() {
          let esc = self.advance();
          text.push('\\');
          text.push(esc);
        }
        continue;
      }
      text.push(c);
      self.advance();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn tokenize(source: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize();
    lexer.tokens.iter().map(|t| t.kind.clone()).collect()
  }

  #[test]
  fn numbers() {
    let tokens = tokenize("42 123.456");
    assert_eq!(tokens, vec![TokenKind::Number(42.0), TokenKind::Number(123.456), TokenKind::Eof]);
  }

  #[test]
  fn strings() {
    let tokens = tokenize("\"hello\" 'world'");
    assert_eq!(
      tokens,
      vec![TokenKind::String("hello".into()), TokenKind::String("world".into()), TokenKind::Eof]
    );
  }

  #[test]
  fn keywords() {
    let tokens = tokenize("true false null undefined let const var");
    assert_eq!(
      tokens,
      vec![
        TokenKind::True,
        TokenKind::False,
        TokenKind::Null,
        TokenKind::Undefined,
        TokenKind::Let,
        TokenKind::Const,
        TokenKind::Var,
        TokenKind::Eof
      ]
    );
  }

  #[test]
  fn operators() {
    let tokens =
      tokenize("+ - * / % ** += -= *= /= = == === != !== < > <= >= && || ?? ! ~ << >> >>> ++ --");
    assert_eq!(
      tokens,
      vec![
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::StarStar,
        TokenKind::PlusEq,
        TokenKind::MinusEq,
        TokenKind::StarEq,
        TokenKind::SlashEq,
        TokenKind::Eq,
        TokenKind::EqEq,
        TokenKind::EqEqEq,
        TokenKind::NotEq,
        TokenKind::NotEqEq,
        TokenKind::Lt,
        TokenKind::Gt,
        TokenKind::LtEq,
        TokenKind::GtEq,
        TokenKind::AmpAmp,
        TokenKind::PipePipe,
        TokenKind::QuestionQuestion,
        TokenKind::Bang,
        TokenKind::Tilde,
        TokenKind::LtLt,
        TokenKind::GtGt,
        TokenKind::GtGtGt,
        TokenKind::PlusPlus,
        TokenKind::MinusMinus,
        TokenKind::Eof
      ]
    );
  }

  #[test]
  fn delimiters() {
    let tokens = tokenize("( ) [ ] { } , ; : . ... =>");
    assert_eq!(
      tokens,
      vec![
        TokenKind::OpenParen,
        TokenKind::CloseParen,
        TokenKind::OpenBracket,
        TokenKind::CloseBracket,
        TokenKind::OpenBrace,
        TokenKind::CloseBrace,
        TokenKind::Comma,
        TokenKind::Semicolon,
        TokenKind::Colon,
        TokenKind::Dot,
        TokenKind::DotDotDot,
        TokenKind::Arrow,
        TokenKind::Eof
      ]
    );
  }

  #[test]
  fn identifier() {
    let tokens = tokenize("foo bar _baz $dollar");
    assert_eq!(
      tokens,
      vec![
        TokenKind::Identifier("foo".into()),
        TokenKind::Identifier("bar".into()),
        TokenKind::Identifier("_baz".into()),
        TokenKind::Identifier("$dollar".into()),
        TokenKind::Eof
      ]
    );
  }

  #[test]
  fn comments_skipped() {
    let tokens = tokenize("42 // line comment\n 3");
    assert_eq!(tokens, vec![TokenKind::Number(42.0), TokenKind::Number(3.0), TokenKind::Eof]);
  }

  #[test]
  fn block_comments_skipped() {
    let tokens = tokenize("42 /* block */ 3");
    assert_eq!(tokens, vec![TokenKind::Number(42.0), TokenKind::Number(3.0), TokenKind::Eof]);
  }

  #[test]
  fn question() {
    let tokens = tokenize("? ?? foo ? bar");
    assert_eq!(
      tokens,
      vec![
        TokenKind::Question,
        TokenKind::QuestionQuestion,
        TokenKind::Identifier("foo".into()),
        TokenKind::Question,
        TokenKind::Identifier("bar".into()),
        TokenKind::Eof
      ]
    );
  }

  #[test]
  fn bitwise_ops() {
    let tokens = tokenize("| ^ &");
    assert_eq!(tokens, vec![TokenKind::Pipe, TokenKind::Caret, TokenKind::Amp, TokenKind::Eof]);
  }

  #[test]
  fn unterminated_string() {
    let mut lexer = Lexer::new("\"hello");
    lexer.tokenize();
    let diags = lexer.into_diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E003");
  }

  #[test]
  fn unknown_char_produces_diagnostic() {
    let mut lexer = Lexer::new("#");
    lexer.tokenize();
    let diags = lexer.into_diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E004");
  }

  #[test]
  fn unknown_char_produces_token() {
    let mut lexer = Lexer::new("#");
    let tokens = lexer.tokenize().to_vec();
    assert!(matches!(tokens[0].kind, TokenKind::Unknown('#')));
  }

  #[test]
  fn nested_block_comment() {
    let tokens = tokenize("1 /* /* inner */ */ 2");
    assert_eq!(tokens, vec![TokenKind::Number(1.0), TokenKind::Number(2.0), TokenKind::Eof]);
  }

  #[test]
  fn amp_eq_token() {
    let tokens = tokenize("&=");
    assert_eq!(tokens, vec![TokenKind::AmpEq, TokenKind::Eof]);
  }

  #[test]
  fn pipe_eq_token() {
    let tokens = tokenize("|=");
    assert_eq!(tokens, vec![TokenKind::PipeEq, TokenKind::Eof]);
  }
}
