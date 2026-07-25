mod expr;
mod stmt;
mod typ;

use crate::ast::{Expression, Program};
use crate::diagnostic::{Diagnostic, SourceFile};
use crate::token::{Token, TokenKind};

pub struct Parser {
  pub tokens: Vec<Token>,
  pub cursor: usize,
  pub source_file: SourceFile,
  diagnostics: Vec<Diagnostic>,
}

impl Parser {
  #[must_use]
  pub fn new(tokens: Vec<Token>, source_file: SourceFile) -> Self {
    Self { tokens, cursor: 0, source_file, diagnostics: Vec::new() }
  }

  pub fn parse(&mut self) -> Program {
    let mut body = Vec::new();
    while !self.is_at_end() {
      if let Some(stmt) = self.parse_statement() {
        body.push(stmt);
      } else {
        self.sync();
      }
    }
    Program { body }
  }

  #[must_use]
  pub fn diagnostics(&self) -> &[Diagnostic] {
    &self.diagnostics
  }

  pub(super) fn sync(&mut self) {
    while !self.is_at_end() {
      match &self.peek().kind {
        TokenKind::Semicolon | TokenKind::CloseBrace | TokenKind::Eof => {
          self.advance();
          return;
        }
        // Stop syncing when we see a token that can start a statement —
        // let the main loop try to parse it instead of skipping over it.
        TokenKind::Let
        | TokenKind::Var
        | TokenKind::Const
        | TokenKind::If
        | TokenKind::While
        | TokenKind::For
        | TokenKind::Return
        | TokenKind::Function
        | TokenKind::OpenBrace
        | TokenKind::Import
        | TokenKind::Type
        | TokenKind::Export
        | TokenKind::Switch
        | TokenKind::Throw
        | TokenKind::Try
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Do
        | TokenKind::Class
        | TokenKind::Enum
        | TokenKind::Interface
        | TokenKind::Async
        | TokenKind::Declare
        | TokenKind::At => {
          return;
        }
        _ => {
          self.advance();
        }
      }
    }
  }

  #[must_use]
  pub fn is_at_end(&self) -> bool {
    self.peek().kind == TokenKind::Eof
  }

  #[must_use]
  pub fn peek(&self) -> &Token {
    let idx = self.cursor.min(self.tokens.len().saturating_sub(1));
    &self.tokens[idx]
  }

  #[must_use]
  pub fn peek_ahead(&self, n: usize) -> &Token {
    let idx = (self.cursor + n).min(self.tokens.len() - 1);
    &self.tokens[idx]
  }

  pub fn advance(&mut self) -> &Token {
    let idx = self.cursor.min(self.tokens.len().saturating_sub(1));
    if self.cursor < self.tokens.len() {
      self.cursor += 1;
    }
    &self.tokens[idx]
  }

  /// Safe accessor for the end span of the last consumed token.
  pub(super) fn last_end(&self) -> usize {
    let idx = self.cursor.saturating_sub(1).min(self.tokens.len().saturating_sub(1));
    self.tokens[idx].span.end
  }

  /// Safe accessor for the start span of the last consumed token.
  pub(super) fn last_start(&self) -> usize {
    let idx = self.cursor.saturating_sub(1).min(self.tokens.len().saturating_sub(1));
    self.tokens[idx].span.start
  }

  pub(super) fn maybe_semicolon(&mut self) {
    if self.peek().kind == TokenKind::Semicolon {
      self.advance();
    }
  }

  pub fn expect(&mut self, kind: TokenKind) -> bool {
    if self.peek().kind == kind {
      self.advance();
      true
    } else {
      let span = self.peek().span;
      self.diagnostics.push(Diagnostic::error(
        "E001",
        format!("Expected '{kind}', found '{}'", self.peek().kind),
        span,
      ));
      false
    }
  }
}

use expr::parse_expression;

impl Parser {
  pub(super) fn parse_expression(&mut self, precedence: u8) -> Option<Box<Expression>> {
    parse_expression(self, precedence)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::*;
  use crate::diagnostic::SourceFile;
  use crate::lexer::Lexer;

  fn parse(source: &str) -> Program {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().to_vec();
    let mut parser = Parser::new(tokens, SourceFile::new("test.ts", source));
    parser.parse()
  }

  fn first_expr(source: &str) -> Box<Expression> {
    let prog = parse(source);
    match &prog.body[0] {
      Statement::ExpressionStatement { expression, .. } => expression.clone(),
      _ => panic!("expected expression statement"),
    }
  }

  #[test]
  fn parse_number() {
    let expr = first_expr("42;");
    assert!(matches!(*expr, Expression::NumberLiteral { value: 42.0, .. }));
  }

  #[test]
  fn parse_string() {
    let expr = first_expr("\"hello\";");
    assert!(matches!(*expr, Expression::StringLiteral { ref value, .. } if value == "hello"));
  }

  #[test]
  fn parse_identifier() {
    let expr = first_expr("foo;");
    assert!(matches!(*expr, Expression::Identifier { ref name, .. } if name == "foo"));
  }

  #[test]
  fn parse_binary() {
    let expr = first_expr("1 + 2;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::Add, .. }));
  }

  #[test]
  fn parse_precedence_mul_over_add() {
    let expr = first_expr("1 + 2 * 3;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::Add, .. }));
    if let Expression::BinaryExpression { left, right, .. } = &*expr {
      assert!(matches!(**left, Expression::NumberLiteral { value: 1.0, .. }));
      assert!(matches!(**right, Expression::BinaryExpression { operator: BinaryOp::Mul, .. }));
    }
  }

  #[test]
  fn parse_precedence_add_over_mul() {
    let expr = first_expr("(1 + 2) * 3;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::Mul, .. }));
    if let Expression::BinaryExpression { left, .. } = &*expr {
      assert!(matches!(**left, Expression::ParenthesizedExpression { .. }));
      if let Expression::ParenthesizedExpression { expression, .. } = &**left {
        assert!(matches!(
          **expression,
          Expression::BinaryExpression { operator: BinaryOp::Add, .. }
        ));
      }
    }
  }

  #[test]
  fn parse_call() {
    let expr = first_expr("foo(1, 2);");
    assert!(matches!(*expr, Expression::CallExpression { .. }));
    if let Expression::CallExpression { callee, arguments, .. } = &*expr {
      assert!(matches!(**callee, Expression::Identifier { ref name, .. } if name == "foo"));
      assert_eq!(arguments.len(), 2);
    }
  }

  #[test]
  fn parse_unary() {
    let expr = first_expr("-42;");
    assert!(matches!(*expr, Expression::UnaryExpression { operator: UnaryOp::Minus, .. }));
  }

  #[test]
  fn parse_unary_not() {
    let expr = first_expr("!true;");
    assert!(matches!(*expr, Expression::UnaryExpression { operator: UnaryOp::Not, .. }));
  }

  #[test]
  fn parse_conditional() {
    let expr = first_expr("a ? b : c;");
    assert!(matches!(*expr, Expression::ConditionalExpression { .. }));
  }

  #[test]
  fn parse_member_dot() {
    let expr = first_expr("a.b;");
    assert!(matches!(*expr, Expression::MemberExpression { computed: false, .. }));
  }

  #[test]
  fn parse_member_bracket() {
    let expr = first_expr("a[0];");
    assert!(matches!(*expr, Expression::MemberExpression { computed: true, .. }));
  }

  #[test]
  fn parse_variable_let() {
    let prog = parse("let x = 42;");
    assert!(matches!(
      &prog.body[0],
      Statement::VariableDeclaration { kind: VariableKind::Let, .. }
    ));
    if let Statement::VariableDeclaration { declarations, .. } = &prog.body[0] {
      assert_eq!(declarations.len(), 1);
      assert!(declarations[0].init.is_some());
    }
  }

  #[test]
  fn parse_variable_no_init() {
    let prog = parse("let x;");
    assert!(matches!(&prog.body[0], Statement::VariableDeclaration { .. }));
    if let Statement::VariableDeclaration { declarations, .. } = &prog.body[0] {
      assert!(declarations[0].init.is_none());
    }
  }

  #[test]
  fn parse_array() {
    let expr = first_expr("[1, 2, 3];");
    assert!(matches!(*expr, Expression::ArrayExpression { .. }));
    if let Expression::ArrayExpression { elements, .. } = &*expr {
      assert_eq!(elements.len(), 3);
    }
  }

  #[test]
  fn parse_object() {
    let expr = first_expr("({a: 1});");
    assert!(matches!(*expr, Expression::ParenthesizedExpression { .. }));
    if let Expression::ParenthesizedExpression { expression, .. } = &*expr {
      assert!(matches!(**expression, Expression::ObjectExpression { .. }));
    }
  }

  #[test]
  fn parse_object_shorthand() {
    let expr = first_expr("({x});");
    assert!(matches!(*expr, Expression::ParenthesizedExpression { .. }));
    if let Expression::ParenthesizedExpression { expression, .. } = &*expr {
      assert!(matches!(**expression, Expression::ObjectExpression { .. }));
    }
  }

  #[test]
  fn parse_assignment() {
    let expr = first_expr("x = 42;");
    assert!(matches!(
      *expr,
      Expression::AssignmentExpression { operator: AssignmentOp::Assign, .. }
    ));
  }

  #[test]
  fn parse_chained_call() {
    let expr = first_expr("a.b(c);");
    assert!(matches!(*expr, Expression::CallExpression { .. }));
    if let Expression::CallExpression { callee, .. } = &*expr {
      assert!(matches!(**callee, Expression::MemberExpression { computed: false, .. }));
    }
  }

  #[test]
  fn parse_comparison() {
    let expr = first_expr("a === b;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::StrictEq, .. }));
  }

  #[test]
  fn parse_logical() {
    let expr = first_expr("a && b || c;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::LogicalOr, .. }));
  }

  #[test]
  fn parse_nullish() {
    let expr = first_expr("a ?? b;");
    assert!(matches!(
      *expr,
      Expression::BinaryExpression { operator: BinaryOp::NullishCoalescing, .. }
    ));
  }

  #[test]
  fn parse_exponentiation() {
    let expr = first_expr("2 ** 3;");
    assert!(matches!(*expr, Expression::BinaryExpression { operator: BinaryOp::Exp, .. }));
  }

  #[test]
  fn parse_parens_preserved() {
    let expr = first_expr("(x);");
    assert!(matches!(*expr, Expression::ParenthesizedExpression { .. }));
  }

  #[test]
  fn parse_error_expected_expression() {
    let src = "+;";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().to_vec();
    let mut parser = Parser::new(tokens, SourceFile::new("test.ts", src));
    let program = parser.parse();
    let diags = parser.diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E002");
    assert_eq!(program.body.len(), 0);
  }

  #[test]
  fn parse_recovers_after_semicolon() {
    let src = "let x = ; let y = 42;";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().to_vec();
    let mut parser = Parser::new(tokens, SourceFile::new("test.ts", src));
    let program = parser.parse();
    assert!(
      program
        .body
        .iter()
        .any(|s| { matches!(s, Statement::VariableDeclaration { kind: VariableKind::Let, .. }) })
    );
  }

  #[test]
  fn parse_typed_variable() {
    let src = "let x: number = 42;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].type_ann.is_some());
        assert_eq!(declarations[0].type_ann, Some(TypeAnn::Number));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_union() {
    let src = "let x: string | number;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Union { .. }));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_array() {
    let src = "let x: number[];";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Array { .. }));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_literal() {
    let src = "let x: 42;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Literal { value: LiteralValue::Number(42.0) }));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_string_literal() {
    let src = "let x: \"hello\";";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Literal { value: LiteralValue::String(s) } if s == "hello"));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_variable_no_type_ann() {
    let src = "let x;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        assert!(declarations[0].type_ann.is_none());
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_boolean_literal() {
    let src = "let x: true;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Literal { value: LiteralValue::Boolean(true) }));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_type_ref() {
    let src = "let x: MyType;";
    let prog = parse(src);
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::TypeRef { name, .. } if name == "MyType"));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_typed_variable_all_keywords() {
    for (keyword, expected) in [
      ("number", TypeAnn::Number),
      ("string", TypeAnn::String),
      ("boolean", TypeAnn::Boolean),
      ("null", TypeAnn::Null),
      ("undefined", TypeAnn::Undefined),
      ("void", TypeAnn::Void),
      ("any", TypeAnn::Any),
      ("never", TypeAnn::Never),
    ] {
      let prog = parse(&format!("let x: {keyword};"));
      match &prog.body[0] {
        Statement::VariableDeclaration { declarations, .. } => {
          assert_eq!(declarations[0].type_ann, Some(expected), "keyword: {keyword}");
        }
        _ => panic!("expected variable declaration for keyword: {keyword}"),
      }
    }
  }

  #[test]
  fn parse_if_basic() {
    let prog = parse("if (x) y;");
    assert!(matches!(&prog.body[0], Statement::IfStatement { .. }));
  }

  #[test]
  fn parse_if_else() {
    let prog = parse("if (x) y; else z;");
    match &prog.body[0] {
      Statement::IfStatement { alternate, .. } => {
        assert!(alternate.is_some());
      }
      _ => panic!("expected if statement"),
    }
  }

  #[test]
  fn parse_if_block_body() {
    let prog = parse("if (x) { y; z; }");
    match &prog.body[0] {
      Statement::IfStatement { consequent, .. } => {
        assert!(matches!(**consequent, Statement::BlockStatement { .. }));
      }
      _ => panic!("expected if statement"),
    }
  }

  #[test]
  fn parse_if_else_if() {
    let prog = parse("if (a) b; else if (c) d; else e;");
    match &prog.body[0] {
      Statement::IfStatement { alternate, .. } => {
        assert!(matches!(alternate.as_deref(), Some(Statement::IfStatement { .. })));
      }
      _ => panic!("expected if statement"),
    }
  }

  #[test]
  fn parse_while_basic() {
    let prog = parse("while (x) y;");
    assert!(matches!(&prog.body[0], Statement::WhileStatement { .. }));
  }

  #[test]
  fn parse_while_block_body() {
    let prog = parse("while (x) { y; }");
    match &prog.body[0] {
      Statement::WhileStatement { body, .. } => {
        assert!(matches!(**body, Statement::BlockStatement { .. }));
      }
      _ => panic!("expected while statement"),
    }
  }

  #[test]
  fn parse_for_basic() {
    let prog = parse("for (let i = 0; i < 10; i = i + 1) x;");
    assert!(matches!(&prog.body[0], Statement::ForStatement { .. }));
  }

  #[test]
  fn parse_for_empty_init_test_update() {
    let prog = parse("for (;;) x;");
    match &prog.body[0] {
      Statement::ForStatement { init, test, update, .. } => {
        assert!(init.is_none());
        assert!(test.is_none());
        assert!(update.is_none());
      }
      _ => panic!("expected for statement"),
    }
  }

  #[test]
  fn parse_for_expr_init() {
    let prog = parse("for (i = 0; i < 10; i++) x;");
    match &prog.body[0] {
      Statement::ForStatement { init, .. } => {
        assert!(matches!(init, Some(ForInit::Expression(_))));
      }
      _ => panic!("expected for statement"),
    }
  }

  #[test]
  fn parse_for_var_init() {
    let prog = parse("for (var i = 0; i < 10; i = i + 1) x;");
    match &prog.body[0] {
      Statement::ForStatement { init, .. } => {
        assert!(matches!(init, Some(ForInit::VariableDeclaration { .. })));
      }
      _ => panic!("expected for statement"),
    }
  }

  #[test]
  fn parse_return() {
    let prog = parse("return 42;");
    assert!(matches!(&prog.body[0], Statement::ReturnStatement { value: Some(_), .. }));
  }

  #[test]
  fn parse_return_no_value() {
    let prog = parse("return;");
    assert!(matches!(&prog.body[0], Statement::ReturnStatement { value: None, .. }));
  }

  #[test]
  fn parse_block() {
    let prog = parse("{ x; y; }");
    match &prog.body[0] {
      Statement::BlockStatement { body, .. } => {
        assert_eq!(body.len(), 2);
      }
      _ => panic!("expected block statement"),
    }
  }

  #[test]
  fn parse_nested_if_while_for() {
    let src = "if (a) { while (b) { for (let i = 0; i < 10; i = i + 1) { return i; } } }";
    let prog = parse(src);
    assert_eq!(prog.body.len(), 1);
    match &prog.body[0] {
      Statement::IfStatement { consequent, .. } => {
        assert!(matches!(**consequent, Statement::BlockStatement { .. }));
      }
      _ => panic!("expected if"),
    }
  }

  #[test]
  fn parse_function_declaration() {
    let prog = parse("function add(a, b) { return a + b; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { name, params, .. } => {
        assert_eq!(name, "add");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_function_no_params() {
    let prog = parse("function noop() { }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { name, params, .. } => {
        assert_eq!(name, "noop");
        assert!(params.is_empty());
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_function_typed_params() {
    let prog = parse("function add(a: number, b: number): number { return a + b; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { name, params, return_type, .. } => {
        assert_eq!(name, "add");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].type_ann, Some(TypeAnn::Number));
        assert_eq!(params[1].type_ann, Some(TypeAnn::Number));
        assert_eq!(*return_type.as_ref().unwrap(), TypeAnn::Number);
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_function_return_type_only() {
    let prog = parse("function getNumber(): number { return 42; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { return_type, .. } => {
        assert_eq!(*return_type.as_ref().unwrap(), TypeAnn::Number);
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_function_type_annotation() {
    let prog = parse("let fn: (number, string) => boolean;");
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let ann = declarations[0].type_ann.as_ref().unwrap();
        assert!(matches!(ann, TypeAnn::Function { params, return_type }
          if params.len() == 2 && matches!(**return_type, TypeAnn::Boolean)));
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_arrow_single_param() {
    let prog = parse("let f = x => x + 1;");
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let init = declarations[0].init.as_ref().unwrap();
        if let Expression::ArrowFunction { params, .. } = init.as_ref() {
          assert_eq!(params.len(), 1);
          assert_eq!(params[0].name, "x");
        } else {
          panic!("expected arrow function");
        }
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_arrow_multi_params() {
    let prog = parse("let f = (x, y) => x + y;");
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let init = declarations[0].init.as_ref().unwrap();
        if let Expression::ArrowFunction { params, .. } = init.as_ref() {
          assert_eq!(params.len(), 2);
        } else {
          panic!("expected arrow function");
        }
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_arrow_block_body() {
    let prog = parse("let f = (x) => { return x; };");
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let init = declarations[0].init.as_ref().unwrap();
        if let Expression::ArrowFunction { body: ArrowFunctionBody::Block(stmts), .. } =
          init.as_ref()
        {
          assert_eq!(stmts.len(), 1);
        } else {
          panic!("expected arrow with block body");
        }
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_arrow_typed_params() {
    let prog = parse("let f = (x: number, y: string) => x;");
    match &prog.body[0] {
      Statement::VariableDeclaration { declarations, .. } => {
        let init = declarations[0].init.as_ref().unwrap();
        match init.as_ref() {
          Expression::ArrowFunction { params, .. } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].type_ann, Some(TypeAnn::Number));
            assert_eq!(params[1].type_ann, Some(TypeAnn::String));
          }
          _ => panic!("expected arrow function"),
        }
      }
      _ => panic!("expected variable declaration"),
    }
  }

  #[test]
  fn parse_parens_still_work() {
    let expr = first_expr("(x);");
    assert!(matches!(*expr, Expression::ParenthesizedExpression { .. }));
  }

  #[test]
  fn parse_parens_binary_still_work() {
    let expr = first_expr("(1 + 2);");
    assert!(matches!(*expr, Expression::ParenthesizedExpression { .. }));
  }

  #[test]
  fn parse_import_named() {
    let prog = parse(r#"import { z } from "zod";"#);
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, source, .. } => {
        assert_eq!(specifiers.len(), 1);
        assert_eq!(specifiers[0].local, "z");
        assert_eq!(source, "zod");
      }
      _ => panic!("expected import declaration"),
    }
  }

  #[test]
  fn parse_import_default() {
    let prog = parse(r#"import React from "react";"#);
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, source, .. } => {
        assert_eq!(specifiers.len(), 1);
        assert_eq!(specifiers[0].local, "React");
        assert_eq!(source, "react");
      }
      _ => panic!("expected import declaration"),
    }
  }

  #[test]
  fn parse_import_as() {
    let prog = parse(r#"import { join as pathJoin } from "path";"#);
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, .. } => {
        assert_eq!(specifiers[0].local, "pathJoin");
        assert_eq!(specifiers[0].imported.as_deref(), Some("join"));
      }
      _ => panic!("expected import declaration"),
    }
  }

  #[test]
  fn parse_import_multiple() {
    let prog = parse(r#"import { a, b, c } from "mod";"#);
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, .. } => {
        assert_eq!(specifiers.len(), 3);
      }
      _ => panic!("expected import declaration"),
    }
  }

  #[test]
  fn parse_import_default_and_named() {
    let prog = parse(r#"import React, { useState } from "react";"#);
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, .. } => {
        assert_eq!(specifiers.len(), 2);
        assert_eq!(specifiers[0].local, "React");
        assert_eq!(specifiers[1].local, "useState");
      }
      _ => panic!("expected import declaration"),
    }
  }

  #[test]
  fn parse_type_alias() {
    let prog = parse("type ID = string;");
    match &prog.body[0] {
      Statement::TypeAliasDeclaration { name, type_annotation, .. } => {
        assert_eq!(name, "ID");
        assert_eq!(*type_annotation, TypeAnn::String);
      }
      _ => panic!("expected type alias"),
    }
  }

  #[test]
  fn parse_type_alias_union() {
    let prog = parse("type Status = string | number;");
    match &prog.body[0] {
      Statement::TypeAliasDeclaration { name, type_annotation, .. } => {
        assert_eq!(name, "Status");
        assert!(matches!(type_annotation, TypeAnn::Union { .. }));
      }
      _ => panic!("expected type alias"),
    }
  }

  #[test]
  fn parse_export_const() {
    let prog = parse("export const x = 42;");
    match &prog.body[0] {
      Statement::ExportDeclaration { declaration, .. } => {
        assert!(matches!(declaration.as_ref(), Statement::VariableDeclaration { .. }));
      }
      _ => panic!("expected export declaration"),
    }
  }

  #[test]
  fn parse_export_default() {
    let prog = parse("export default 42;");
    match &prog.body[0] {
      Statement::ExportDeclaration { declaration, .. } => {
        assert!(matches!(declaration.as_ref(), Statement::ExpressionStatement { .. }));
      }
      _ => panic!("expected export declaration"),
    }
  }

  #[test]
  fn parse_error_txt_pattern() {
    let source = r#"import { z } from "zod"

const s = z.object({
  name: z.string(),
})

type S = string

const a: S = "Hello"
"#;
    let prog = parse(source);
    assert_eq!(prog.body.len(), 4, "expected 4 statements, got {}", prog.body.len());
    assert!(matches!(&prog.body[0], Statement::ImportDeclaration { .. }));
    assert!(matches!(&prog.body[1], Statement::VariableDeclaration { .. }));
    assert!(matches!(&prog.body[2], Statement::TypeAliasDeclaration { .. }));
    assert!(matches!(&prog.body[3], Statement::VariableDeclaration { .. }));
  }

  #[test]
  fn parse_for_of() {
    let prog = parse("for (const x of items) { x; }");
    assert!(matches!(&prog.body[0], Statement::ForInOfStatement { is_of: true, .. }));
  }

  #[test]
  fn parse_for_in() {
    let prog = parse("for (const x in obj) { x; }");
    assert!(matches!(&prog.body[0], Statement::ForInOfStatement { is_of: false, .. }));
  }

  #[test]
  fn parse_for_of_let() {
    let prog = parse("for (let x of items) { x; }");
    match &prog.body[0] {
      Statement::ForInOfStatement { kind, left, is_of, .. } => {
        assert_eq!(*kind, VariableKind::Let);
        assert!(*is_of);
        if let Expression::Identifier { name, .. } = left {
          assert_eq!(name, "x");
        } else {
          panic!("expected identifier");
        }
      }
      _ => panic!("expected for-of"),
    }
  }

  #[test]
  fn parse_for_in_var() {
    let prog = parse("for (var x in obj) { x; }");
    match &prog.body[0] {
      Statement::ForInOfStatement { kind, left, is_of, .. } => {
        assert_eq!(*kind, VariableKind::Var);
        assert!(!*is_of);
        if let Expression::Identifier { name, .. } = left {
          assert_eq!(name, "x");
        } else {
          panic!("expected identifier");
        }
      }
      _ => panic!("expected for-in"),
    }
  }

  #[test]
  fn parse_for_bare_of() {
    let prog = parse("for (x of arr) { x; }");
    match &prog.body[0] {
      Statement::ForInOfStatement { kind, left, .. } => {
        if let Expression::Identifier { name, .. } = left {
          assert_eq!(name, "x");
        } else {
          panic!("expected identifier");
        }
        assert_eq!(*kind, VariableKind::Var);
      }
      _ => panic!("expected for-of"),
    }
  }

  #[test]
  fn parse_switch_basic() {
    let prog = parse("switch (x) { case 1: break; }");
    assert!(matches!(&prog.body[0], Statement::SwitchStatement { .. }));
  }

  #[test]
  fn parse_switch_with_default() {
    let prog = parse("switch (x) { case 1: y; break; default: z; break; }");
    match &prog.body[0] {
      Statement::SwitchStatement { cases, .. } => {
        assert_eq!(cases.len(), 2);
        assert!(cases[0].test.is_some());
        assert!(cases[1].test.is_none());
      }
      _ => panic!("expected switch"),
    }
  }

  #[test]
  fn parse_throw() {
    let prog = parse("throw new Error(\"msg\");");
    assert!(matches!(&prog.body[0], Statement::ThrowStatement { .. }));
  }

  #[test]
  fn parse_throw_literal() {
    let prog = parse("throw 42;");
    match &prog.body[0] {
      Statement::ThrowStatement { argument, .. } => {
        assert!(matches!(argument.as_ref(), Expression::NumberLiteral { value: 42.0, .. }));
      }
      _ => panic!("expected throw"),
    }
  }

  #[test]
  fn parse_try_catch() {
    let prog = parse("try { x; } catch (e) { y; }");
    match &prog.body[0] {
      Statement::TryStatement { handler, finalizer, .. } => {
        assert!(handler.is_some());
        assert!(finalizer.is_none());
        assert_eq!(handler.as_ref().unwrap().param.as_deref(), Some("e"));
      }
      _ => panic!("expected try"),
    }
  }

  #[test]
  fn parse_try_finally() {
    let prog = parse("try { x; } finally { z; }");
    match &prog.body[0] {
      Statement::TryStatement { handler, finalizer, .. } => {
        assert!(handler.is_none());
        assert!(finalizer.is_some());
      }
      _ => panic!("expected try"),
    }
  }

  #[test]
  fn parse_try_catch_finally() {
    let prog = parse("try { x; } catch (e) { y; } finally { z; }");
    match &prog.body[0] {
      Statement::TryStatement { handler, finalizer, .. } => {
        assert!(handler.is_some());
        assert!(finalizer.is_some());
      }
      _ => panic!("expected try"),
    }
  }

  #[test]
  fn parse_try_catch_typed_param() {
    let prog = parse("try { x; } catch (e: Error) { y; }");
    match &prog.body[0] {
      Statement::TryStatement { handler, .. } => {
        let h = handler.as_ref().expect("handler");
        assert_eq!(h.param.as_deref(), Some("e"));
        assert!(h.type_ann.is_some());
      }
      _ => panic!("expected try"),
    }
  }

  #[test]
  fn parse_default_params() {
    let prog = parse("function f(a = 1, b = \"hello\") { a; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { params, .. } => {
        assert_eq!(params.len(), 2);
        assert!(params[0].default_value.is_some());
        assert!(params[1].default_value.is_some());
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_rest_param() {
    let prog = parse("function f(...args) { args; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { params, .. } => {
        assert_eq!(params.len(), 1);
        assert!(params[0].is_rest);
        assert_eq!(params[0].name, "args");
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_rest_param_with_typed() {
    let prog = parse("function f(a: number, ...rest: string[]) { rest; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { params, .. } => {
        assert_eq!(params.len(), 2);
        assert!(!params[0].is_rest);
        assert!(params[0].type_ann.is_some());
        assert!(params[1].is_rest);
        assert!(params[1].type_ann.is_some());
      }
      _ => panic!("expected function declaration"),
    }
  }

  #[test]
  fn parse_amp_eq_assignment() {
    let expr = first_expr("x &= 3;");
    assert!(matches!(
      *expr,
      Expression::AssignmentExpression { operator: AssignmentOp::BitAndAssign, .. }
    ));
  }

  #[test]
  fn parse_pipe_eq_assignment() {
    let expr = first_expr("x |= 3;");
    assert!(matches!(
      *expr,
      Expression::AssignmentExpression { operator: AssignmentOp::BitOrAssign, .. }
    ));
  }

  #[test]
  fn parse_object_type_literal() {
    let prog = parse("type Foo = { name: string, age: number };");
    match &prog.body[0] {
      Statement::TypeAliasDeclaration { name, type_annotation, .. } => {
        assert_eq!(name, "Foo");
        match type_annotation {
          TypeAnn::Object { properties } => {
            assert_eq!(properties.len(), 2);
            assert_eq!(properties[0].name, "name");
            assert_eq!(properties[0].type_ann, TypeAnn::String);
            assert_eq!(properties[1].name, "age");
            assert_eq!(properties[1].type_ann, TypeAnn::Number);
          }
          other => panic!("expected Object type, got {other:?}"),
        }
      }
      _ => panic!("expected type alias"),
    }
  }

  #[test]
  fn parse_object_type_literal_single() {
    let prog = parse("function f(x: { value: number }) { x; }");
    match &prog.body[0] {
      Statement::FunctionDeclaration { params, .. } => {
        let ref ann = params[0].type_ann;
        assert!(
          matches!(ann, Some(TypeAnn::Object { properties }) if properties.len() == 1 && properties[0].name == "value" && properties[0].type_ann == TypeAnn::Number)
        );
      }
      other => panic!("expected function, got {other:?}"),
    }
  }

  #[test]
  fn parse_import_type() {
    let prog = parse("import type { Foo } from \"bar\";");
    match &prog.body[0] {
      Statement::ImportDeclaration { specifiers, source, is_type, .. } => {
        assert!(*is_type);
        assert_eq!(specifiers.len(), 1);
        assert_eq!(specifiers[0].local, "Foo");
        assert_eq!(source, "bar");
      }
      _ => panic!("expected import type declaration"),
    }
  }

  #[test]
  fn parse_import_type_default() {
    let prog = parse("import type Foo from \"bar\";");
    match &prog.body[0] {
      Statement::ImportDeclaration { is_type, source, .. } => {
        assert!(*is_type);
        assert_eq!(source, "bar");
      }
      _ => panic!("expected import type declaration"),
    }
  }

  #[test]
  fn parse_template_literal_type_simple() {
    let prog = parse("type Greeting = `hello ${string}`;");
    match &prog.body[0] {
      Statement::TypeAliasDeclaration { type_annotation, .. } => match type_annotation {
        TypeAnn::TemplateLiteral { quasis, types } => {
          assert_eq!(quasis, &vec!["hello ".to_string(), "".to_string()]);
          assert_eq!(types.len(), 1);
          assert_eq!(types[0], TypeAnn::String);
        }
        other => panic!("expected TemplateLiteral type, got {other:?}"),
      },
      _ => panic!("expected type alias"),
    }
  }

  #[test]
  fn parse_template_literal_type_multi() {
    let prog = parse("type ID = `${string}-${number}-${boolean}`;");
    match &prog.body[0] {
      Statement::TypeAliasDeclaration { type_annotation, .. } => match type_annotation {
        TypeAnn::TemplateLiteral { quasis, types } => {
          assert_eq!(
            quasis,
            &vec!["".to_string(), "-".to_string(), "-".to_string(), "".to_string()]
          );
          assert_eq!(types.len(), 3);
          assert_eq!(types[0], TypeAnn::String);
          assert_eq!(types[1], TypeAnn::Number);
          assert_eq!(types[2], TypeAnn::Boolean);
        }
        other => panic!("expected TemplateLiteral type, got {other:?}"),
      },
      _ => panic!("expected type alias"),
    }
  }
}
