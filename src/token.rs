use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
  pub start: usize,
  pub end: usize,
}

impl Span {
  #[must_use]
  pub fn new(start: usize, end: usize) -> Self {
    Self { start, end }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
  // Literals
  Number(f64),
  String(String),
  True,
  False,
  Null,
  Undefined,

  // Identifiers & keywords
  Identifier(String),
  Let,
  Const,
  Var,
  Function,
  Return,
  If,
  Else,
  For,
  While,
  Class,
  TypeOf,
  KeyOf,
  Void,
  Delete,
  Import,
  From,
  Export,
  As,
  Default,
  Type,
  Switch,
  Case,
  Throw,
  Try,
  Catch,
  Finally,
  Of,
  In,
  Break,
  Continue,
  New,
  Do,
  This,
  Super,
  Extends,
  Static,
  Async,
  Await,
  Enum,
  Interface,
  Instanceof,
  Public,
  Private,
  Protected,
  Satisfies,
  Meta,
  Declare,
  Module,
  Readonly,

  // Operators (ordered by precedence in comments)
  // 1: ??
  QuestionQuestion,
  QuestionDot,
  // 2: ||
  PipePipe,
  // 3: &&
  AmpAmp,
  // 4: |
  Pipe,
  // 5: ^
  Caret,
  // 6: &
  Amp,
  // 7: == != === !==
  EqEq,
  NotEq,
  EqEqEq,
  NotEqEq,
  // 8: < > <= >=
  Lt,
  Gt,
  LtEq,
  GtEq,
  // 9: << >> >>>
  LtLt,
  GtGt,
  GtGtGt,
  // 10: + -
  Plus,
  Minus,
  // 11: * / %
  Star,
  Slash,
  Percent,
  // 12: **
  StarStar,

  // Unary operators
  Bang,
  Tilde,

  // Update operators
  PlusPlus,
  MinusMinus,

  // Assignment
  Eq,
  PlusEq,
  MinusEq,
  StarEq,
  SlashEq,
  PercentEq,
  AmpEq,
  PipeEq,
  QuestionQuestionEq,
  AmpAmpEq,
  PipePipeEq,
  CaretEq,

  // Punctuation
  OpenParen,
  CloseParen,
  OpenBracket,
  CloseBracket,
  OpenBrace,
  CloseBrace,
  Comma,
  Semicolon,
  Colon,
  Dot,
  Question,
  Arrow,
  DotDotDot,

  // Template literals
  NoSubstitutionTemplate(String),
  TemplateHead(String),
  TemplateMiddle(String),
  TemplateTail(String),

  // Decorators
  At,

  // Special
  Unknown(char),
  Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
  pub kind: TokenKind,
  pub span: Span,
}

impl Token {
  #[must_use]
  pub fn new(kind: TokenKind, span: Span) -> Self {
    Self { kind, span }
  }
}

impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TokenKind::Number(n) => write!(f, "{n}"),
      TokenKind::String(s) => write!(f, "\"{s}\""),
      TokenKind::True => write!(f, "true"),
      TokenKind::False => write!(f, "false"),
      TokenKind::Null => write!(f, "null"),
      TokenKind::Undefined => write!(f, "undefined"),
      TokenKind::Identifier(s) => write!(f, "{s}"),
      TokenKind::Let => write!(f, "let"),
      TokenKind::Const => write!(f, "const"),
      TokenKind::Var => write!(f, "var"),
      TokenKind::Function => write!(f, "function"),
      TokenKind::Return => write!(f, "return"),
      TokenKind::If => write!(f, "if"),
      TokenKind::Else => write!(f, "else"),
      TokenKind::For => write!(f, "for"),
      TokenKind::While => write!(f, "while"),
      TokenKind::Class => write!(f, "class"),
      TokenKind::TypeOf => write!(f, "typeof"),
      TokenKind::KeyOf => write!(f, "keyof"),
      TokenKind::Void => write!(f, "void"),
      TokenKind::Delete => write!(f, "delete"),
      TokenKind::Import => write!(f, "import"),
      TokenKind::From => write!(f, "from"),
      TokenKind::Export => write!(f, "export"),
      TokenKind::As => write!(f, "as"),
      TokenKind::Default => write!(f, "default"),
      TokenKind::Type => write!(f, "type"),
      TokenKind::Switch => write!(f, "switch"),
      TokenKind::Case => write!(f, "case"),
      TokenKind::Throw => write!(f, "throw"),
      TokenKind::Try => write!(f, "try"),
      TokenKind::Catch => write!(f, "catch"),
      TokenKind::Finally => write!(f, "finally"),
      TokenKind::Of => write!(f, "of"),
      TokenKind::In => write!(f, "in"),
      TokenKind::Break => write!(f, "break"),
      TokenKind::Continue => write!(f, "continue"),
      TokenKind::New => write!(f, "new"),
      TokenKind::Do => write!(f, "do"),
      TokenKind::This => write!(f, "this"),
      TokenKind::Super => write!(f, "super"),
      TokenKind::Extends => write!(f, "extends"),
      TokenKind::Static => write!(f, "static"),
      TokenKind::Async => write!(f, "async"),
      TokenKind::Await => write!(f, "await"),
      TokenKind::Enum => write!(f, "enum"),
      TokenKind::Interface => write!(f, "interface"),
      TokenKind::Instanceof => write!(f, "instanceof"),
      TokenKind::Public => write!(f, "public"),
      TokenKind::Private => write!(f, "private"),
      TokenKind::Protected => write!(f, "protected"),
      TokenKind::Satisfies => write!(f, "satisfies"),
      TokenKind::Meta => write!(f, "meta"),
      TokenKind::Declare => write!(f, "declare"),
      TokenKind::Module => write!(f, "module"),
      TokenKind::Readonly => write!(f, "readonly"),
      TokenKind::QuestionQuestion => write!(f, "??"),
      TokenKind::QuestionDot => write!(f, "?."),
      TokenKind::PipePipe => write!(f, "||"),
      TokenKind::AmpAmp => write!(f, "&&"),
      TokenKind::Pipe => write!(f, "|"),
      TokenKind::Caret => write!(f, "^"),
      TokenKind::Amp => write!(f, "&"),
      TokenKind::EqEq => write!(f, "=="),
      TokenKind::NotEq => write!(f, "!="),
      TokenKind::EqEqEq => write!(f, "==="),
      TokenKind::NotEqEq => write!(f, "!=="),
      TokenKind::Lt => write!(f, "<"),
      TokenKind::Gt => write!(f, ">"),
      TokenKind::LtEq => write!(f, "<="),
      TokenKind::GtEq => write!(f, ">="),
      TokenKind::LtLt => write!(f, "<<"),
      TokenKind::GtGt => write!(f, ">>"),
      TokenKind::GtGtGt => write!(f, ">>>"),
      TokenKind::Plus => write!(f, "+"),
      TokenKind::Minus => write!(f, "-"),
      TokenKind::Star => write!(f, "*"),
      TokenKind::Slash => write!(f, "/"),
      TokenKind::Percent => write!(f, "%"),
      TokenKind::StarStar => write!(f, "**"),
      TokenKind::Bang => write!(f, "!"),
      TokenKind::Tilde => write!(f, "~"),
      TokenKind::PlusPlus => write!(f, "++"),
      TokenKind::MinusMinus => write!(f, "--"),
      TokenKind::Eq => write!(f, "="),
      TokenKind::PlusEq => write!(f, "+="),
      TokenKind::MinusEq => write!(f, "-="),
      TokenKind::StarEq => write!(f, "*="),
      TokenKind::SlashEq => write!(f, "/="),
      TokenKind::PercentEq => write!(f, "%="),
      TokenKind::AmpEq => write!(f, "&="),
      TokenKind::PipeEq => write!(f, "|="),
      TokenKind::QuestionQuestionEq => write!(f, "??="),
      TokenKind::AmpAmpEq => write!(f, "&&="),
      TokenKind::PipePipeEq => write!(f, "||="),
      TokenKind::CaretEq => write!(f, "^="),
      TokenKind::OpenParen => write!(f, "("),
      TokenKind::CloseParen => write!(f, ")"),
      TokenKind::OpenBracket => write!(f, "["),
      TokenKind::CloseBracket => write!(f, "]"),
      TokenKind::OpenBrace => write!(f, "{{"),
      TokenKind::CloseBrace => write!(f, "}}"),
      TokenKind::Comma => write!(f, ","),
      TokenKind::Semicolon => write!(f, ";"),
      TokenKind::Colon => write!(f, ":"),
      TokenKind::Dot => write!(f, "."),
      TokenKind::Question => write!(f, "?"),
      TokenKind::Arrow => write!(f, "=>"),
      TokenKind::DotDotDot => write!(f, "..."),
      TokenKind::NoSubstitutionTemplate(s) => write!(f, "`{s}`"),
      TokenKind::TemplateHead(s) => write!(f, "`{s} ${{"),
      TokenKind::TemplateMiddle(s) => write!(f, "}} {s} ${{"),
      TokenKind::TemplateTail(s) => write!(f, "}} {s}`"),
      TokenKind::At => write!(f, "@"),
      TokenKind::Unknown(c) => write!(f, "unknown char '{c}'"),
      TokenKind::Eof => write!(f, "EOF"),
    }
  }
}

impl TokenKind {
  /// True for keywords that can appear as property keys in object literals: `{ class: 1 }`.
  pub fn is_reserved_for_property_key(&self) -> bool {
    matches!(
      self,
      TokenKind::If
        | TokenKind::Else
        | TokenKind::Return
        | TokenKind::Function
        | TokenKind::Class
        | TokenKind::New
        | TokenKind::Delete
        | TokenKind::TypeOf
        | TokenKind::Void
        | TokenKind::Instanceof
        | TokenKind::In
        | TokenKind::Of
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::Default
        | TokenKind::Switch
        | TokenKind::Case
        | TokenKind::This
        | TokenKind::Super
        | TokenKind::Extends
        | TokenKind::Static
        | TokenKind::Throw
        | TokenKind::Try
        | TokenKind::Catch
        | TokenKind::Finally
        | TokenKind::For
        | TokenKind::While
        | TokenKind::Do
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Async
        | TokenKind::Await
        | TokenKind::Let
        | TokenKind::Const
        | TokenKind::Var
        | TokenKind::Type
        | TokenKind::Enum
        | TokenKind::Interface
        | TokenKind::Declare
        | TokenKind::Module
        | TokenKind::As
        | TokenKind::From
        | TokenKind::Readonly
        | TokenKind::Satisfies
    )
  }
}
