use crate::ast::{
  CatchClause, ClassBody, ClassField, EnumMember, Expression, ForInit, ImportSpecifier,
  MethodDefinition, MethodKind, ObjectPatternProperty, Parameter, PropertyKey, Statement,
  SwitchCase, TypeAnn, VariableDeclarator, VariableKind,
};
use crate::diagnostic::Diagnostic;
use crate::token::{Span, TokenKind};

use super::Parser;

impl Parser {
  pub(super) fn parse_statement(&mut self) -> Option<Statement> {
    match &self.peek().kind {
      TokenKind::Let | TokenKind::Var => Some(self.parse_variable_declaration()),
      TokenKind::Const => {
        // Could be const enum
        if self.peek_ahead(1).kind == TokenKind::Enum {
          Some(self.parse_const_enum_declaration())
        } else {
          Some(self.parse_variable_declaration())
        }
      }
      TokenKind::If => Some(self.parse_if_statement()),
      TokenKind::While => Some(self.parse_while_statement()),
      TokenKind::For => Some(self.parse_for_statement()),
      TokenKind::Return => Some(self.parse_return_statement()),
      TokenKind::Function => Some(self.parse_function_declaration()),
      TokenKind::OpenBrace => Some(self.parse_block()),
      TokenKind::Import => Some(self.parse_import_statement()),
      TokenKind::Type => Some(self.parse_type_alias()),
      TokenKind::Export => Some(self.parse_export_declaration()),
      TokenKind::Switch => Some(self.parse_switch_statement()),
      TokenKind::Throw => Some(self.parse_throw_statement()),
      TokenKind::Try => Some(self.parse_try_statement()),
      TokenKind::Break => Some(self.parse_break_statement()),
      TokenKind::Continue => Some(self.parse_continue_statement()),
      TokenKind::Do => Some(self.parse_do_while_statement()),
      TokenKind::Class => Some(self.parse_class_declaration()),
      TokenKind::Enum => Some(self.parse_enum_declaration()),
      TokenKind::Interface => Some(self.parse_interface_declaration()),
      TokenKind::Async => Some(self.parse_function_declaration()),
      TokenKind::Declare => Some(self.parse_declare_statement()),
      TokenKind::At => {
        let mut decorators = Vec::new();
        while self.peek().kind == TokenKind::At {
          self.advance(); // consume @
          let expr = self
            .parse_expression(0)
            .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
          decorators.push(*expr);
        }
        let inner = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
          expression: Box::new(Expression::Placeholder { span: self.peek().span }),
          span: self.peek().span,
        });
        let start = decorators.first().map_or(inner.span().start, |e| e.span().start);
        let end = inner.span().end;
        Some(Statement::DecoratedStatement {
          decorators,
          statement: Box::new(inner),
          span: Span::new(start, end),
        })
      }
      _ => {
        // Labeled statement: identifier followed by ':'
        if let TokenKind::Identifier(_) = &self.peek().kind
          && self.peek_ahead(1).kind == TokenKind::Colon
        {
          let start = self.peek().span.start;
          let label = if let TokenKind::Identifier(n) = &self.peek().kind {
            n.clone()
          } else {
            String::new()
          };
          self.advance(); // consume identifier
          self.advance(); // consume ':'
          let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
            expression: Box::new(Expression::Placeholder { span: self.peek().span }),
            span: self.peek().span,
          });
          let end = body.span().end;
          return Some(Statement::LabeledStatement {
            label,
            body: Box::new(body),
            span: Span::new(start, end),
          });
        }
        let expr = self.parse_expression(0);
        if let Some(expr) = expr {
          let span = expr.span();
          self.maybe_semicolon();
          Some(Statement::ExpressionStatement { expression: expr, span })
        } else {
          let span = self.peek().span;
          self.diagnostics.push(Diagnostic::error("E002", "Expected expression", span));
          None
        }
      }
    }
  }

  pub(super) fn parse_block(&mut self) -> Statement {
    let start = self.advance().span.start;
    let mut body = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      if let Some(stmt) = self.parse_statement() {
        body.push(stmt);
      } else {
        self.sync();
      }
    }
    let end = self.expect(TokenKind::CloseBrace);
    let end_pos = if end { self.tokens[self.cursor - 1].span.end } else { self.last_end() };
    Statement::BlockStatement { body, span: Span::new(start, end_pos) }
  }

  pub(super) fn parse_if_statement(&mut self) -> Statement {
    let start = self.advance().span.start;
    self.expect(TokenKind::OpenParen);
    let test = self
      .parse_expression(0)
      .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
    self.expect(TokenKind::CloseParen);
    let consequent = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    let alternate = if self.peek().kind == TokenKind::Else {
      self.advance();
      self.parse_statement().map(Box::new)
    } else {
      None
    };
    let end = alternate.as_ref().map_or(consequent.span(), |s| s.span());
    Statement::IfStatement {
      test,
      consequent: Box::new(consequent),
      alternate,
      span: Span::new(start, end.end),
    }
  }

  pub(super) fn parse_while_statement(&mut self) -> Statement {
    let start = self.advance().span.start;
    self.expect(TokenKind::OpenParen);
    let test = self
      .parse_expression(0)
      .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
    self.expect(TokenKind::CloseParen);
    let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    let end = body.span();
    Statement::WhileStatement { test, body: Box::new(body), span: Span::new(start, end.end) }
  }

  pub(super) fn parse_for_statement(&mut self) -> Statement {
    let start = self.advance().span.start;
    self.expect(TokenKind::OpenParen);

    // Detect `for await (let/const/var x of expr)`
    let is_await = if self.peek().kind == TokenKind::Await {
      self.advance(); // consume 'await'
      true
    } else {
      false
    };

    // Detect for-in / for-of: for (let/const/var x of/in expr)
    // Also handle destructured patterns: for (const { a, b } of expr)
    if matches!(self.peek().kind, TokenKind::Let | TokenKind::Const | TokenKind::Var)
      && (matches!(self.peek_ahead(2).kind, TokenKind::Of | TokenKind::In)
        || matches!(self.peek_ahead(1).kind, TokenKind::OpenBrace | TokenKind::OpenBracket))
    {
      let kind = match &self.peek().kind {
        TokenKind::Let => VariableKind::Let,
        TokenKind::Const => VariableKind::Const,
        TokenKind::Var => VariableKind::Var,
        _ => {
          self.diagnostics.push(Diagnostic::error(
            "E999",
            "Internal parser error: unexpected state",
            self.peek().span,
          ));
          VariableKind::Var
        }
      };
      self.advance(); // consume let/const/var
      let left = if matches!(self.peek().kind, TokenKind::OpenBrace | TokenKind::OpenBracket) {
        // Destructured pattern: { a, b } or [a, b]
        self.parse_lvalue()
      } else {
        let left_span = self.peek().span;
        let left_name = if let TokenKind::Identifier(n) = &self.peek().kind {
          let n = n.clone();
          self.advance();
          n
        } else {
          self.diagnostics.push(Diagnostic::error(
            "E001",
            format!("Expected identifier, found '{}'", self.peek().kind),
            self.peek().span,
          ));
          String::new()
        };
        Box::new(Expression::Identifier { name: left_name, span: left_span })
      };
      let is_of = self.peek().kind == TokenKind::Of;
      self.advance(); // consume of/in
      let right = self
        .parse_expression(0)
        .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
      self.expect(TokenKind::CloseParen);
      let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
        expression: Box::new(Expression::Placeholder { span: self.peek().span }),
        span: self.peek().span,
      });
      let end = body.span();
      return Statement::ForInOfStatement {
        kind,
        left: *left,
        right,
        body: Box::new(body),
        is_of,
        is_await,
        span: Span::new(start, end.end),
      };
    }

    // Also check bare for-in/for-of: for (x of/in expr) — x is an existing identifier
    if let TokenKind::Identifier(_) = &self.peek().kind
      && matches!(self.peek_ahead(1).kind, TokenKind::Of | TokenKind::In)
    {
      let left_span = self.peek().span;
      let left_name = if let TokenKind::Identifier(n) = &self.peek().kind {
        let n = n.clone();
        self.advance();
        n
      } else {
        self.diagnostics.push(Diagnostic::error(
          "E999",
          "Internal parser error: unexpected state",
          self.peek().span,
        ));
        String::new()
      };
      let is_of = self.peek().kind == TokenKind::Of;
      self.advance(); // consume of/in
      let right = self
        .parse_expression(0)
        .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
      self.expect(TokenKind::CloseParen);
      let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
        expression: Box::new(Expression::Placeholder { span: self.peek().span }),
        span: self.peek().span,
      });
      let end = body.span();
      return Statement::ForInOfStatement {
        kind: VariableKind::Var,
        left: Expression::Identifier { name: left_name, span: left_span },
        right,
        body: Box::new(body),
        is_of,
        is_await,
        span: Span::new(start, end.end),
      };
    }

    // init
    let init = match &self.peek().kind {
      TokenKind::Semicolon => {
        self.advance();
        None
      }
      TokenKind::Let | TokenKind::Const | TokenKind::Var => {
        let decl = self.parse_variable_declaration_for_init();
        match decl {
          Statement::VariableDeclaration { kind, declarations, .. } => {
            Some(ForInit::VariableDeclaration { kind, declarations })
          }
          _ => {
            self.diagnostics.push(Diagnostic::error(
              "E999",
              "Internal parser error: unexpected state in for-init",
              self.peek().span,
            ));
            Some(ForInit::VariableDeclaration { kind: VariableKind::Var, declarations: Vec::new() })
          }
        }
      }
      _ => {
        let expr = self
          .parse_expression(0)
          .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
        self.maybe_semicolon();
        Some(ForInit::Expression(expr))
      }
    };

    // test
    let test =
      if self.peek().kind == TokenKind::Semicolon { None } else { self.parse_expression(0) };
    self.maybe_semicolon();

    // update
    let update =
      if self.peek().kind == TokenKind::CloseParen { None } else { self.parse_expression(0) };
    self.expect(TokenKind::CloseParen);

    let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    let end = body.span();
    Statement::ForStatement {
      init,
      test,
      update,
      body: Box::new(body),
      span: Span::new(start, end.end),
    }
  }

  pub(super) fn parse_variable_declaration_for_init(&mut self) -> Statement {
    let kind = match &self.peek().kind {
      TokenKind::Let => VariableKind::Let,
      TokenKind::Const => VariableKind::Const,
      TokenKind::Var => VariableKind::Var,
      _ => {
        self.diagnostics.push(Diagnostic::error(
          "E999",
          "Internal parser error: unexpected state",
          self.peek().span,
        ));
        VariableKind::Var
      }
    };
    self.advance();
    let mut declarations = Vec::new();
    loop {
      let id = self.parse_lvalue();
      let type_ann =
        if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
      let init = if self.peek().kind == TokenKind::Eq {
        self.advance();
        self.parse_expression(0)
      } else {
        None
      };
      let span = Span::new(id.span().start, init.as_ref().map_or(id.span().end, |e| e.span().end));
      declarations.push(VariableDeclarator { id, type_ann, init, span });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      } else {
        break;
      }
    }
    self.maybe_semicolon();
    Statement::VariableDeclaration { kind, declarations, span: self.peek().span }
  }

  pub(super) fn parse_return_statement(&mut self) -> Statement {
    let start = self.advance().span.start;
    let value =
      if self.peek().kind == TokenKind::Semicolon { None } else { self.parse_expression(0) };
    self.maybe_semicolon();
    let end = value.as_ref().map_or(self.last_end(), |e| e.span().end);
    Statement::ReturnStatement { value, span: Span::new(start, end) }
  }

  pub(super) fn parse_function_declaration(&mut self) -> Statement {
    let is_async = self.peek().kind == TokenKind::Async;
    let start = self.advance().span.start;
    // consume 'function' if we just consumed 'async'
    if is_async {
      if self.peek().kind == TokenKind::Function {
        self.advance();
      } else {
        // async not followed by function — backtrack? For now, just continue
      }
    }
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      n.clone()
    } else {
      self.diagnostics.push(Diagnostic::error(
        "E001",
        format!("Expected identifier, found '{}'", self.peek().kind),
        self.peek().span,
      ));
      String::new()
    };
    if !name.is_empty() {
      self.advance();
    }
    let type_params = self.parse_type_params();
    self.expect(TokenKind::OpenParen);
    let params = self.parse_function_params();
    self.expect(TokenKind::CloseParen);
    let return_type =
      if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
    let body = self.parse_block();
    let end = body.span();
    Statement::FunctionDeclaration {
      name,
      params,
      return_type,
      body: Box::new(body),
      is_async,
      type_params,
      span: Span::new(start, end.end),
    }
  }

  pub(super) fn parse_function_params(&mut self) -> Vec<Parameter> {
    let mut params = Vec::new();
    while self.peek().kind != TokenKind::CloseParen && !self.is_at_end() {
      let start = self.peek().span;
      let is_rest = self.peek().kind == TokenKind::DotDotDot;
      if is_rest {
        self.advance();
      }
      let name = if let TokenKind::Identifier(n) = &self.peek().kind {
        n.clone()
      } else {
        self.advance();
        continue;
      };
      self.advance();
      let type_ann =
        if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
      let default_value = if self.peek().kind == TokenKind::Eq {
        self.advance();
        self.parse_expression(0)
      } else {
        None
      };
      let end = self.last_end();
      params.push(Parameter {
        name,
        type_ann,
        default_value,
        is_rest,
        span: Span::new(start.start, end),
      });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      }
    }
    params
  }

  pub(super) fn parse_type_params(&mut self) -> Vec<(String, Option<TypeAnn>)> {
    if self.peek().kind != TokenKind::Lt {
      return Vec::new();
    }
    self.advance(); // consume '<'
    let mut params = Vec::new();
    while self.peek().kind != TokenKind::Gt && !self.is_at_end() {
      if let TokenKind::Identifier(n) = &self.peek().kind {
        let name = n.clone();
        self.advance();
        // Optional: extends Type
        let constraint = if self.peek().kind == TokenKind::Extends {
          self.advance();
          self.parse_type()
        } else {
          None
        };
        params.push((name, constraint));
      } else {
        break;
      }
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      }
    }
    if self.peek().kind == TokenKind::Gt {
      self.advance(); // consume '>'
    }
    params
  }

  pub(super) fn parse_variable_declaration(&mut self) -> Statement {
    let start = self.advance().span.start;
    let kind = match &self.tokens[self.cursor - 1].kind {
      TokenKind::Let => VariableKind::Let,
      TokenKind::Const => VariableKind::Const,
      TokenKind::Var => VariableKind::Var,
      _ => {
        self.diagnostics.push(Diagnostic::error(
          "E999",
          "Internal parser error: unexpected state",
          self.peek().span,
        ));
        VariableKind::Var
      }
    };
    let mut declarations = Vec::new();
    loop {
      let id = self.parse_lvalue();
      let type_ann =
        if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
      let init = if self.peek().kind == TokenKind::Eq {
        self.advance();
        self.parse_expression(0)
      } else {
        None
      };
      let span = Span::new(id.span().start, init.as_ref().map_or(id.span().end, |e| e.span().end));
      declarations.push(VariableDeclarator { id, type_ann, init, span });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      } else {
        break;
      }
    }
    self.maybe_semicolon();
    let end = self.last_end();
    Statement::VariableDeclaration { kind, declarations, span: Span::new(start, end) }
  }

  pub(super) fn parse_import_statement(&mut self) -> Statement {
    let start = self.advance().span.start;
    let is_type = if self.peek().kind == TokenKind::Type {
      self.advance();
      true
    } else {
      false
    };
    let mut specifiers = Vec::new();
    if self.peek().kind == TokenKind::OpenBrace {
      // import { x, y as z } from "mod"
      self.advance();
      while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
        let local_start = self.peek().span;
        let name = if let TokenKind::Identifier(n) = &self.peek().kind {
          n.clone()
        } else {
          self.advance();
          continue;
        };
        self.advance();
        let (local, imported) = if self.peek().kind == TokenKind::As {
          self.advance();
          if let TokenKind::Identifier(alias) = &self.peek().kind {
            let a = alias.clone();
            self.advance();
            (a, Some(name))
          } else {
            (name, None)
          }
        } else {
          (name.clone(), None)
        };
        let end = self.last_end();
        specifiers.push(ImportSpecifier {
          local,
          imported,
          span: Span::new(local_start.start, end),
          is_default: false,
          is_namespace: false,
        });
        if self.peek().kind == TokenKind::Comma {
          self.advance();
        }
      }
      self.expect(TokenKind::CloseBrace);
    } else if self.peek().kind == TokenKind::Star {
      // import * as x from "mod"
      self.advance();
      self.expect(TokenKind::As);
      let local = if let TokenKind::Identifier(n) = &self.peek().kind {
        let n = n.clone();
        self.advance();
        n
      } else {
        String::new()
      };
      let end = self.last_end();
      specifiers.push(ImportSpecifier {
        local,
        imported: None,
        span: Span::new(start, end),
        is_default: false,
        is_namespace: true,
      });
    } else if matches!(self.peek().kind, TokenKind::Identifier(_)) {
      // import x from "mod"  or  import x, { y } from "mod"
      let name =
        if let TokenKind::Identifier(n) = &self.peek().kind { n.clone() } else { String::new() };
      self.advance();
      let end = self.last_end();
      specifiers.push(ImportSpecifier {
        local: name,
        imported: None,
        span: Span::new(start, end),
        is_default: true,
        is_namespace: false,
      });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
        if self.peek().kind == TokenKind::OpenBrace {
          self.advance();
          while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
            let local_start = self.peek().span;
            let n = if let TokenKind::Identifier(id) = &self.peek().kind {
              id.clone()
            } else {
              self.advance();
              continue;
            };
            self.advance();
            let (local, imported) = if self.peek().kind == TokenKind::As {
              self.advance();
              if let TokenKind::Identifier(alias) = &self.peek().kind {
                let a = alias.clone();
                self.advance();
                (a, Some(n))
              } else {
                (n, None)
              }
            } else {
              (n.clone(), None)
            };
            let end = self.last_end();
            specifiers.push(ImportSpecifier {
              local,
              imported,
              span: Span::new(local_start.start, end),
              is_default: false,
              is_namespace: false,
            });
            if self.peek().kind == TokenKind::Comma {
              self.advance();
            }
          }
          self.expect(TokenKind::CloseBrace);
        }
      }
    }
    self.expect(TokenKind::From);
    let source = if let TokenKind::String(s) = &self.peek().kind {
      let s = s.clone();
      self.advance();
      s
    } else {
      self.expect(TokenKind::String(String::new()));
      String::new()
    };
    self.maybe_semicolon();
    let end = self.last_end();
    Statement::ImportDeclaration { specifiers, source, is_type, span: Span::new(start, end) }
  }

  pub(super) fn parse_type_alias(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'type'
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      n
    } else {
      self.diagnostics.push(Diagnostic::error(
        "E001",
        format!("Expected identifier, found '{}'", self.peek().kind),
        self.peek().span,
      ));
      String::new()
    };
    let type_params = self.parse_type_params();
    self.expect(TokenKind::Eq);
    let type_annotation = self.parse_type().unwrap_or(TypeAnn::Any);
    self.maybe_semicolon();
    let end = self.last_end();
    Statement::TypeAliasDeclaration {
      name,
      type_params,
      type_annotation,
      span: Span::new(start, end),
    }
  }

  pub(super) fn parse_export_declaration(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'export'
    // export type { ... } / export type function/class/const — skip 'type' keyword (TS-only)
    if self.peek().kind == TokenKind::Type
      && matches!(
        self.peek_ahead(1).kind,
        TokenKind::OpenBrace
          | TokenKind::Function
          | TokenKind::Class
          | TokenKind::Const
          | TokenKind::Let
          | TokenKind::Var
          | TokenKind::Default
      )
    {
      self.advance();
      // export type { ... } — skip the entire type-only export
      if self.peek().kind == TokenKind::OpenBrace {
        // consume up to matching brace
        self.advance();
        while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
          self.advance();
        }
        if self.peek().kind == TokenKind::CloseBrace {
          self.advance();
        }
        // optional from clause
        if self.peek().kind == TokenKind::From {
          self.advance();
          if matches!(&self.peek().kind, TokenKind::String(_)) {
            self.advance();
          }
        }
        self.maybe_semicolon();
        let end = self.last_end();
        return Statement::ExportDeclaration {
          declaration: Box::new(Statement::ExpressionStatement {
            expression: Box::new(Expression::Placeholder { span: Span::new(start, end) }),
            span: Span::new(start, end),
          }),
          is_default: false,
          span: Span::new(start, end),
        };
      }
    }
    // export * from "source"
    if self.peek().kind == TokenKind::Star {
      self.advance(); // consume *
      self.expect(TokenKind::From);
      let source = if let TokenKind::String(s) = &self.peek().kind {
        let s = s.clone();
        self.advance();
        s
      } else {
        self.expect(TokenKind::String(String::new()));
        String::new()
      };
      self.maybe_semicolon();
      let end = self.last_end();
      return Statement::ExportDeclaration {
        declaration: Box::new(Statement::ImportDeclaration {
          specifiers: vec![],
          source,
          is_type: false,
          span: Span::new(start, end),
        }),
        is_default: false,
        span: Span::new(start, end),
      };
    }
    if self.peek().kind == TokenKind::Default {
      self.advance();
      // export default function() { ... } — anonymous function
      if self.peek().kind == TokenKind::Function
        && matches!(self.tokens.get(self.cursor + 1).map(|t| &t.kind), Some(TokenKind::OpenParen))
      {
        let fn_start = self.advance().span.start; // consume 'function'
        self.expect(TokenKind::OpenParen);
        let params = self.parse_function_params();
        self.expect(TokenKind::CloseParen);
        let return_type =
          if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
        let body = self.parse_block();
        let end = body.span();
        let decl = Statement::FunctionDeclaration {
          name: String::new(),
          params,
          return_type,
          body: Box::new(body),
          is_async: false,
          type_params: Vec::new(),
          span: Span::new(fn_start, end.end),
        };
        return Statement::ExportDeclaration {
          declaration: Box::new(decl),
          is_default: true,
          span: Span::new(start, end.end),
        };
      }
      // export default { ... } — parse as expression, not block statement
      if self.peek().kind == TokenKind::OpenBrace {
        let expr = self
          .parse_expression(0)
          .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
        self.maybe_semicolon();
        let end = expr.span().end;
        let expr_stmt =
          Statement::ExpressionStatement { expression: expr, span: Span::new(start, end) };
        return Statement::ExportDeclaration {
          declaration: Box::new(expr_stmt),
          is_default: true,
          span: Span::new(start, end),
        };
      }
      let decl = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
        expression: Box::new(Expression::Placeholder { span: self.peek().span }),
        span: self.peek().span,
      });
      let end = decl.span();
      return Statement::ExportDeclaration {
        declaration: Box::new(decl),
        is_default: true,
        span: Span::new(start, end.end),
      };
    }
    if self.peek().kind == TokenKind::OpenBrace {
      // export { x, y } — skip to matching brace, emit as expression statement placeholder
      let depth_start = self.peek().span.start;
      self.advance();
      let mut names = Vec::new();
      while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
        if let TokenKind::Identifier(n) = &self.peek().kind {
          names.push(n.clone());
        }
        self.advance();
      }
      self.expect(TokenKind::CloseBrace);
      // optional "from" clause
      if self.peek().kind == TokenKind::From {
        self.advance();
        if let TokenKind::String(_) = &self.peek().kind {
          self.advance();
        }
      }
      self.maybe_semicolon();
      let end = self.last_end();
      // Re-export: wrap as a no-op expression for now
      let expr = Expression::Placeholder { span: Span::new(start, end) };
      return Statement::ExportDeclaration {
        declaration: Box::new(Statement::ExpressionStatement {
          expression: Box::new(expr),
          span: Span::new(depth_start, end),
        }),
        is_default: false,
        span: Span::new(start, end),
      };
    }
    // export const/let/var/function/class → parse inner, wrap
    let decl = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    let end = decl.span();
    Statement::ExportDeclaration {
      declaration: Box::new(decl),
      is_default: false,
      span: Span::new(start, end.end),
    }
  }

  pub(super) fn parse_switch_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'switch'
    self.expect(TokenKind::OpenParen);
    let discriminant = self
      .parse_expression(0)
      .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
    self.expect(TokenKind::CloseParen);
    self.expect(TokenKind::OpenBrace);
    let mut cases = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      let case_start = self.peek().span.start;
      if self.peek().kind == TokenKind::Case {
        self.advance(); // consume 'case'
        let test = Some(
          self
            .parse_expression(0)
            .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span })),
        );
        self.expect(TokenKind::Colon);
        let mut body = Vec::new();
        while self.peek().kind != TokenKind::Case
          && self.peek().kind != TokenKind::Default
          && self.peek().kind != TokenKind::CloseBrace
          && !self.is_at_end()
        {
          if let Some(stmt) = self.parse_statement() {
            body.push(stmt);
          } else {
            self.sync();
          }
        }
        let end = body.last().map_or(self.last_end(), |s| s.span().end);
        cases.push(SwitchCase { test, body, span: Span::new(case_start, end) });
      } else if self.peek().kind == TokenKind::Default {
        self.advance(); // consume 'default'
        self.expect(TokenKind::Colon);
        let mut body = Vec::new();
        while self.peek().kind != TokenKind::Case
          && self.peek().kind != TokenKind::Default
          && self.peek().kind != TokenKind::CloseBrace
          && !self.is_at_end()
        {
          if let Some(stmt) = self.parse_statement() {
            body.push(stmt);
          } else {
            self.sync();
          }
        }
        let end = body.last().map_or(self.last_end(), |s| s.span().end);
        cases.push(SwitchCase { test: None, body, span: Span::new(case_start, end) });
      } else {
        // skip unexpected token inside switch
        self.advance();
      }
    }
    self.expect(TokenKind::CloseBrace);
    let end = self.last_end();
    Statement::SwitchStatement { discriminant, cases, span: Span::new(start, end) }
  }

  pub(super) fn parse_throw_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'throw'
    let argument = self
      .parse_expression(0)
      .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
    self.maybe_semicolon();
    let end = argument.span().end;
    Statement::ThrowStatement { argument, span: Span::new(start, end) }
  }

  pub(super) fn parse_break_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'break'
    let label = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      Some(n)
    } else {
      None
    };
    self.maybe_semicolon();
    Statement::BreakStatement { label, span: Span::new(start, self.last_end()) }
  }

  pub(super) fn parse_continue_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'continue'
    let label = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      Some(n)
    } else {
      None
    };
    self.maybe_semicolon();
    Statement::ContinueStatement { label, span: Span::new(start, self.last_end()) }
  }

  pub(super) fn parse_do_while_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'do'
    let body = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    self.expect(TokenKind::While);
    self.expect(TokenKind::OpenParen);
    let test = self
      .parse_expression(0)
      .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
    self.expect(TokenKind::CloseParen);
    self.maybe_semicolon();
    let end = self.last_end();
    Statement::DoWhileStatement { test, body: Box::new(body), span: Span::new(start, end) }
  }

  pub(super) fn parse_try_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'try'
    let body = self.parse_block();
    let handler = if self.peek().kind == TokenKind::Catch {
      let catch_start = self.advance().span.start;
      // Bare catch (ES2019): catch { ... } without parameter
      if self.peek().kind == TokenKind::OpenBrace {
        let block = self.parse_block();
        match block {
          Statement::BlockStatement { body, .. } => {
            let end = body.last().map_or(self.last_end(), |s| s.span().end);
            Some(CatchClause {
              param: None,
              type_ann: None,
              body,
              span: Span::new(catch_start, end),
            })
          }
          _ => {
            self.diagnostics.push(Diagnostic::error(
              "E999",
              "Internal parser error: unexpected state in catch block",
              self.peek().span,
            ));
            None
          }
        }
      } else {
        self.expect(TokenKind::OpenParen);
        let param = if let TokenKind::Identifier(n) = &self.peek().kind {
          let n = n.clone();
          self.advance();
          Some(n)
        } else {
          None
        };
        let type_ann = if self.peek().kind == TokenKind::Colon {
          self.advance(); // consume ':'
          self.parse_type()
        } else {
          None
        };
        self.expect(TokenKind::CloseParen);
        let block = self.parse_block();
        match block {
          Statement::BlockStatement { body, .. } => {
            let end = body.last().map_or(self.last_end(), |s| s.span().end);
            Some(CatchClause { param, type_ann, body, span: Span::new(catch_start, end) })
          }
          _ => {
            self.diagnostics.push(Diagnostic::error(
              "E999",
              "Internal parser error: unexpected state in catch block",
              self.peek().span,
            ));
            None
          }
        }
      }
    } else {
      None
    };
    let finalizer = if self.peek().kind == TokenKind::Finally {
      self.advance(); // consume 'finally'
      let block = self.parse_block();
      match block {
        Statement::BlockStatement { body, .. } => Some(body),
        _ => {
          self.diagnostics.push(Diagnostic::error(
            "E999",
            "Internal parser error: unexpected state in finally block",
            self.peek().span,
          ));
          None
        }
      }
    } else {
      None
    };
    let end = finalizer
      .as_ref()
      .and_then(|b| b.last())
      .map_or_else(|| handler.as_ref().map_or(self.last_end(), |h| h.span.end), |s| s.span().end);
    Statement::TryStatement {
      body: Box::new(body),
      handler,
      finalizer,
      span: Span::new(start, end),
    }
  }

  pub(super) fn parse_class_declaration(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'class'
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      n
    } else {
      self.diagnostics.push(Diagnostic::error(
        "E001",
        format!("Expected identifier, found '{}'", self.peek().kind),
        self.peek().span,
      ));
      String::new()
    };
    // Optional: extends Expr
    let superclass = if self.peek().kind == TokenKind::Extends {
      self.advance();
      self.parse_expression(0)
    } else {
      None
    };
    // Body
    self.expect(TokenKind::OpenBrace);
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      // Parse optional visibility modifier
      let visibility = match self.peek().kind {
        TokenKind::Public => {
          self.advance();
          Some(crate::ast::Visibility::Public)
        }
        TokenKind::Private => {
          self.advance();
          Some(crate::ast::Visibility::Private)
        }
        TokenKind::Protected => {
          self.advance();
          Some(crate::ast::Visibility::Protected)
        }
        _ => None,
      };
      let is_static = if self.peek().kind == TokenKind::Static {
        self.advance();
        true
      } else {
        false
      };
      // Peek at key: constructor, method, or field
      let key = match &self.peek().kind {
        TokenKind::Identifier(n) => {
          let key = PropertyKey::Identifier(n.clone());
          self.advance();
          key
        }
        TokenKind::String(n) => {
          let key = PropertyKey::String(n.clone());
          self.advance();
          key
        }
        TokenKind::OpenBracket => {
          self.advance();
          let expr = self.parse_expression(0);
          self.expect(TokenKind::CloseBracket);
          if let Some(e) = expr {
            PropertyKey::Expression(e)
          } else {
            self.advance();
            continue;
          }
        }
        _ => {
          self.advance();
          continue;
        }
      };
      // If next is '(', it's a method. Otherwise, it's a field.
      if self.peek().kind == TokenKind::OpenParen {
        let kind = match &key {
          PropertyKey::Identifier(n) if n == "constructor" => MethodKind::Constructor,
          _ => MethodKind::Method,
        };
        self.expect(TokenKind::OpenParen);
        let params = self.parse_function_params();
        self.expect(TokenKind::CloseParen);
        let return_type =
          if self.peek().kind == TokenKind::Colon { self.parse_type_annotation() } else { None };
        let body = self.parse_block();
        let body_end = body.span();
        methods.push(MethodDefinition {
          key,
          kind,
          params,
          return_type,
          body: Box::new(body),
          is_static,
          visibility,
          span: Span::new(start, body_end.end),
        });
      } else {
        // Field declaration: optional type annotation, optional initializer, then semicolon
        let type_ann = if self.peek().kind == TokenKind::Colon {
          self.advance();
          self.parse_type()
        } else {
          None
        };
        let init = if self.peek().kind == TokenKind::Eq {
          self.advance();
          self.parse_expression(0)
        } else {
          None
        };
        // Consume optional semicolon
        if self.peek().kind == TokenKind::Semicolon {
          self.advance();
        }
        let field_end = self.last_end();
        fields.push(ClassField {
          key,
          type_ann,
          init,
          is_static,
          visibility,
          span: Span::new(start, field_end),
        });
      }
    }
    let body_end = self.expect(TokenKind::CloseBrace);
    let end = if body_end { self.tokens[self.cursor - 1].span.end } else { self.last_end() };
    Statement::ClassDeclaration {
      name,
      superclass,
      body: ClassBody { methods, fields, span: Span::new(start, end) },
      span: Span::new(start, end),
    }
  }

  pub(super) fn parse_enum_declaration(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'enum'
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      n
    } else {
      String::new()
    };
    self.expect(TokenKind::OpenBrace);
    let mut members = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      let member_start = self.peek().span;
      let member_name = if let TokenKind::Identifier(n) = &self.peek().kind {
        let n = n.clone();
        self.advance();
        n
      } else {
        self.advance();
        continue;
      };
      let value = if self.peek().kind == TokenKind::Eq {
        self.advance();
        if let TokenKind::String(s) = &self.peek().kind {
          let s = s.clone();
          self.advance();
          Some(s)
        } else if let TokenKind::Number(n) = &self.peek().kind {
          let s = (*n as i64).to_string();
          self.advance();
          Some(s)
        } else {
          None
        }
      } else {
        None
      };
      let end = self.last_end();
      members.push(EnumMember {
        name: member_name,
        value,
        span: Span::new(member_start.start, end),
      });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      }
    }
    self.expect(TokenKind::CloseBrace);
    let end = self.last_end();
    Statement::EnumDeclaration { name, members, span: Span::new(start, end) }
  }

  pub(super) fn parse_const_enum_declaration(&mut self) -> Statement {
    self.advance(); // consume 'const'
    let start = self.tokens[self.cursor - 1].span.start;
    self.advance(); // consume 'enum'
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      n
    } else {
      String::new()
    };
    self.expect(TokenKind::OpenBrace);
    let mut members = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      let member_start = self.peek().span;
      let member_name = if let TokenKind::Identifier(n) = &self.peek().kind {
        let n = n.clone();
        self.advance();
        n
      } else {
        self.advance();
        continue;
      };
      let value = if self.peek().kind == TokenKind::Eq {
        self.advance();
        if let TokenKind::String(s) = &self.peek().kind {
          let s = s.clone();
          self.advance();
          Some(s)
        } else if let TokenKind::Number(n) = &self.peek().kind {
          let s = (*n as i64).to_string();
          self.advance();
          Some(s)
        } else {
          None
        }
      } else {
        None
      };
      let end = self.last_end();
      members.push(EnumMember {
        name: member_name,
        value,
        span: Span::new(member_start.start, end),
      });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      }
    }
    self.expect(TokenKind::CloseBrace);
    let end = self.last_end();
    Statement::EnumDeclaration { name, members, span: Span::new(start, end) }
  }

  pub(super) fn parse_declare_statement(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'declare'
    // declare module "name" { ... }
    if self.peek().kind == TokenKind::Module {
      self.advance(); // consume 'module'
      let name = if let TokenKind::String(s) = &self.peek().kind {
        let s = s.clone();
        self.advance();
        s
      } else if let TokenKind::Identifier(n) = &self.peek().kind {
        let n = n.clone();
        self.advance();
        n
      } else {
        String::new()
      };
      let body = if self.peek().kind == TokenKind::OpenBrace {
        let block = self.parse_block();
        match block {
          Statement::BlockStatement { body, .. } => body,
          _ => {
            self.diagnostics.push(Diagnostic::error(
              "E999",
              "Internal parser error: unexpected state in declare module",
              self.peek().span,
            ));
            Vec::new()
          }
        }
      } else {
        self.maybe_semicolon();
        Vec::new()
      };
      let end = self.last_end();
      return Statement::DeclareModule { name, body, span: Span::new(start, end) };
    }
    // declare namespace Name { ... }
    if let TokenKind::Identifier(n) = &self.peek().kind {
      if n == "namespace" {
        self.advance(); // consume 'namespace'
        let name = if let TokenKind::Identifier(n) = &self.peek().kind {
          let n = n.clone();
          self.advance();
          n
        } else {
          String::new()
        };
        let body = if self.peek().kind == TokenKind::OpenBrace {
          let block = self.parse_block();
          match block {
            Statement::BlockStatement { body, .. } => body,
            _ => {
              self.diagnostics.push(Diagnostic::error(
                "E999",
                "Internal parser error: unexpected state in declare namespace",
                self.peek().span,
              ));
              Vec::new()
            }
          }
        } else {
          self.maybe_semicolon();
          Vec::new()
        };
        let end = self.last_end();
        return Statement::DeclareNamespace { name, body, span: Span::new(start, end) };
      }
    }
    // Fallback: just skip remaining statement
    let decl = self.parse_statement().unwrap_or_else(|| Statement::ExpressionStatement {
      expression: Box::new(Expression::Placeholder { span: self.peek().span }),
      span: self.peek().span,
    });
    let end = decl.span();
    Statement::ExportDeclaration {
      declaration: Box::new(decl),
      is_default: false,
      span: Span::new(start, end.end),
    }
  }

  pub(super) fn parse_interface_declaration(&mut self) -> Statement {
    let start = self.advance().span.start; // consume 'interface'
    let name = if let TokenKind::Identifier(n) = &self.peek().kind {
      let n = n.clone();
      self.advance();
      n
    } else {
      String::new()
    };
    // Skip type params if present: <T, U extends V>
    if self.peek().kind == TokenKind::Lt {
      let mut depth = 1i32;
      self.advance();
      while depth > 0 && !self.is_at_end() {
        match &self.peek().kind {
          TokenKind::Lt => depth += 1,
          TokenKind::Gt => depth -= 1,
          _ => {}
        }
        self.advance();
      }
    }
    // Skip extends clause(s)
    while self.peek().kind == TokenKind::Extends {
      self.advance();
      // Skip until `{` — consume one type expression
      while self.peek().kind != TokenKind::OpenBrace && !self.is_at_end() {
        self.advance();
      }
    }
    // Skip entire body `{ ... }` by brace counting
    if self.peek().kind == TokenKind::OpenBrace {
      let mut depth = 1i32;
      self.advance();
      while depth > 0 && !self.is_at_end() {
        match &self.peek().kind {
          TokenKind::OpenBrace => depth += 1,
          TokenKind::CloseBrace => depth -= 1,
          _ => {}
        }
        if depth > 0 {
          self.advance();
        }
      }
      if self.peek().kind == TokenKind::CloseBrace {
        self.advance();
      }
    }
    self.maybe_semicolon();
    let end = self.last_end();
    Statement::InterfaceDeclaration { name, span: Span::new(start, end) }
  }

  /// Parse lvalue: identifier, object pattern, or array pattern.
  pub(super) fn parse_lvalue(&mut self) -> Box<Expression> {
    match &self.peek().kind {
      TokenKind::OpenBrace => self.parse_object_pattern(),
      TokenKind::OpenBracket => self.parse_array_pattern(),
      _ => self.parse_expression(14).unwrap_or_else(|| {
        Box::new(Expression::Identifier { name: String::new(), span: self.peek().span })
      }),
    }
  }

  pub(super) fn parse_object_pattern(&mut self) -> Box<Expression> {
    let start = self.advance().span.start; // consume {
    let mut properties = Vec::new();
    while self.peek().kind != TokenKind::CloseBrace && !self.is_at_end() {
      let prop_start = self.peek().span;
      let computed = self.peek().kind == TokenKind::OpenBracket;
      let key = if computed {
        self.advance();
        let key_expr = self
          .parse_expression(0)
          .unwrap_or_else(|| Box::new(Expression::Placeholder { span: self.peek().span }));
        self.expect(TokenKind::CloseBracket);
        PropertyKey::Expression(key_expr)
      } else {
        match &self.peek().kind {
          TokenKind::Identifier(n) => {
            let n = n.clone();
            self.advance();
            PropertyKey::Identifier(n)
          }
          TokenKind::String(s) => {
            let s = s.clone();
            self.advance();
            PropertyKey::String(s)
          }
          _ => {
            self.advance();
            continue;
          }
        }
      };
      // : pattern (nested) or shorthand
      let (value, shorthand) = if self.peek().kind == TokenKind::Colon {
        self.advance();
        (self.parse_lvalue(), false)
      } else {
        // shorthand: { a } means { a: a }
        let name = match &key {
          PropertyKey::Identifier(n) => n.clone(),
          _ => String::new(),
        };
        (Box::new(Expression::Identifier { name, span: prop_start }), true)
      };
      let end = self.last_end();
      properties.push(ObjectPatternProperty {
        key,
        value,
        shorthand,
        span: Span::new(prop_start.start, end),
      });
      if self.peek().kind == TokenKind::Comma {
        self.advance();
      }
    }
    let end = self.expect(TokenKind::CloseBrace);
    let end_pos = if end { self.tokens[self.cursor - 1].span.end } else { self.last_end() };
    Box::new(Expression::ObjectPattern { properties, span: Span::new(start, end_pos) })
  }

  pub(super) fn parse_array_pattern(&mut self) -> Box<Expression> {
    let start = self.advance().span.start; // consume [
    let mut elements = Vec::new();
    while self.peek().kind != TokenKind::CloseBracket && !self.is_at_end() {
      if self.peek().kind == TokenKind::Comma {
        elements.push(None); // hole
        self.advance();
      } else if self.peek().kind == TokenKind::CloseBracket {
        break;
      } else {
        elements.push(Some(self.parse_lvalue()));
        if self.peek().kind == TokenKind::Comma {
          self.advance();
        }
      }
    }
    let end = self.expect(TokenKind::CloseBracket);
    let end_pos = if end { self.tokens[self.cursor - 1].span.end } else { self.last_end() };
    Box::new(Expression::ArrayPattern { elements, span: Span::new(start, end_pos) })
  }
}
