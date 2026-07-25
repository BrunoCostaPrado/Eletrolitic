use crate::ast::{
  ArrowFunctionBody, AssignmentOp, BinaryOp, Expression, Parameter, Property, PropertyKey,
  Statement, TypeAnn, UnaryOp,
};
use crate::diagnostic::Diagnostic;
use crate::parser::Parser;
use crate::token::{Span, Token, TokenKind};

pub(crate) fn get_infix_led(kind: &TokenKind) -> Option<(u8, bool)> {
  match kind {
    TokenKind::QuestionQuestion => Some((1, true)),
    TokenKind::PipePipe => Some((2, false)),
    TokenKind::AmpAmp => Some((3, false)),
    TokenKind::Pipe => Some((4, false)),
    TokenKind::Caret => Some((5, false)),
    TokenKind::Amp => Some((6, false)),
    TokenKind::EqEq | TokenKind::NotEq | TokenKind::EqEqEq | TokenKind::NotEqEq => Some((7, false)),
    TokenKind::Instanceof => Some((7, false)),
    TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq => Some((8, false)),
    TokenKind::LtLt | TokenKind::GtGt | TokenKind::GtGtGt => Some((9, false)),
    TokenKind::Plus | TokenKind::Minus => Some((10, false)),
    TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((11, false)),
    TokenKind::StarStar => Some((12, true)),
    TokenKind::Eq
    | TokenKind::PlusEq
    | TokenKind::MinusEq
    | TokenKind::StarEq
    | TokenKind::SlashEq
    | TokenKind::PercentEq
    | TokenKind::AmpEq
    | TokenKind::PipeEq
    | TokenKind::QuestionQuestionEq
    | TokenKind::AmpAmpEq
    | TokenKind::PipePipeEq
    | TokenKind::CaretEq => Some((1, true)),
    _ => None,
  }
}

pub fn parse_expression(p: &mut Parser, min_prec: u8) -> Option<Box<Expression>> {
  let mut left = parse_prefix(p)?;
  loop {
    if p.is_at_end() {
      break;
    }
    let kind = p.peek().kind.clone();

    // Handle postfix operators
    if kind == TokenKind::PlusPlus || kind == TokenKind::MinusMinus {
      if 16 >= min_prec {
        p.advance();
        let span = Span::new(left.span().start, p.last_end());
        let op = if kind == TokenKind::PlusPlus { UnaryOp::PlusPlus } else { UnaryOp::MinusMinus };
        left = Box::new(Expression::UnaryExpression { operator: op, operand: left, span });
        continue;
      }
      break;
    }

    // Handle call expressions
    if kind == TokenKind::OpenParen {
      if 17 >= min_prec {
        p.advance();
        let args = parse_arguments(p);
        let span = Span::new(left.span().start, p.last_end());
        left = Box::new(Expression::CallExpression {
          callee: left,
          arguments: args,
          optional: false,
          span,
        });
        continue;
      }
      break;
    }

    // Handle member expressions
    if kind == TokenKind::Dot || kind == TokenKind::OpenBracket {
      if 18 >= min_prec {
        let computed = kind == TokenKind::OpenBracket;
        p.advance();
        let (property, span) = if computed {
          let expr = parse_expression(p, 0)?;
          p.expect(TokenKind::CloseBracket);
          let end = p.last_end();
          (expr, Span::new(left.span().start, end))
        } else {
          let name =
            if let TokenKind::Identifier(n) = &p.peek().kind { n.clone() } else { String::new() };
          p.advance();
          let end = p.last_end();
          (
            Box::new(Expression::Identifier { name, span: Span::new(end, end) }),
            Span::new(left.span().start, end),
          )
        };
        left = Box::new(Expression::MemberExpression {
          object: left,
          property,
          computed,
          optional: false,
          span,
        });
        continue;
      }
      break;
    }

    // Handle optional chaining: ?.prop, ?.[expr], ?.(args)
    if kind == TokenKind::QuestionDot {
      let next = p.peek_ahead(1).kind.clone();
      if next == TokenKind::OpenBracket {
        // ?.[expr]
        if 18 >= min_prec {
          p.advance(); // ?.
          p.advance(); // [
          let prop = parse_expression(p, 0)?;
          p.expect(TokenKind::CloseBracket);
          let end = p.last_end();
          let span = Span::new(left.span().start, end);
          left = Box::new(Expression::MemberExpression {
            object: left,
            property: prop,
            computed: true,
            optional: true,
            span,
          });
          continue;
        }
        break;
      } else if next == TokenKind::OpenParen {
        // ?.(args)
        if 17 >= min_prec {
          p.advance(); // ?.
          p.advance(); // (
          let args = parse_arguments(p);
          let span = Span::new(left.span().start, p.last_end());
          left = Box::new(Expression::CallExpression {
            callee: left,
            arguments: args,
            optional: true,
            span,
          });
          continue;
        }
        break;
      }
      // ?.property
      if 18 >= min_prec {
        p.advance(); // ?.
        let name =
          if let TokenKind::Identifier(n) = &p.peek().kind { n.clone() } else { String::new() };
        p.advance();
        let end = p.last_end();
        let property = Box::new(Expression::Identifier { name, span: Span::new(end, end) });
        let span = Span::new(left.span().start, end);
        left = Box::new(Expression::MemberExpression {
          object: left,
          property,
          computed: false,
          optional: true,
          span,
        });
        continue;
      }
      break;
    }

    // Handle template literal after expression (tagged template): expr`...`
    if matches!(kind, TokenKind::NoSubstitutionTemplate(_) | TokenKind::TemplateHead(_)) {
      // Tagged template literal
      let mut quasis = Vec::new();
      let mut expressions = Vec::new();
      match &kind {
        TokenKind::NoSubstitutionTemplate(text) => {
          quasis.push(text.clone());
          p.advance();
          let span = Span::new(left.span().start, p.last_end());
          left =
            Box::new(Expression::TaggedTemplateLiteral { tag: left, quasis, expressions, span });
          continue;
        }
        TokenKind::TemplateHead(text) => {
          quasis.push(text.clone());
          p.advance();
          if let Some(expr) = parse_expression(p, 0) {
            expressions.push(expr);
          }
          loop {
            match &p.peek().kind {
              TokenKind::TemplateMiddle(t) => {
                quasis.push(t.clone());
                p.advance();
                if let Some(expr) = parse_expression(p, 0) {
                  expressions.push(expr);
                }
              }
              TokenKind::TemplateTail(t) => {
                quasis.push(t.clone());
                p.advance();
                let span = Span::new(left.span().start, p.last_end());
                left = Box::new(Expression::TaggedTemplateLiteral {
                  tag: left,
                  quasis,
                  expressions,
                  span,
                });
                break;
              }
              _ => {
                let span = Span::new(left.span().start, p.peek().span.start);
                left = Box::new(Expression::TaggedTemplateLiteral {
                  tag: left,
                  quasis,
                  expressions,
                  span,
                });
                break;
              }
            }
          }
          continue;
        }
        _ => {
          p.diagnostics.push(Diagnostic::error(
            "E999",
            "Internal parser error: unexpected state in template literal",
            p.peek().span,
          ));
          break;
        }
      }
    }
    // Handle ternary (?:)
    if kind == TokenKind::Question && p.peek_ahead(1).kind != TokenKind::Question {
      if 1 >= min_prec {
        p.advance();
        let consequent = parse_expression(p, 0)?;
        p.expect(TokenKind::Colon);
        let alternate = parse_expression(p, 1)?;
        let span = Span::new(left.span().start, alternate.span().end);
        left =
          Box::new(Expression::ConditionalExpression { test: left, consequent, alternate, span });
        continue;
      }
      break;
    }

    // Infix operators
    if let Some((prec, is_right)) = get_infix_led(&kind) {
      if prec < min_prec {
        break;
      }
      let is_assign = matches!(
        &kind,
        TokenKind::Eq
          | TokenKind::PlusEq
          | TokenKind::MinusEq
          | TokenKind::StarEq
          | TokenKind::SlashEq
          | TokenKind::PercentEq
          | TokenKind::AmpEq
          | TokenKind::PipeEq
          | TokenKind::QuestionQuestionEq
          | TokenKind::AmpAmpEq
          | TokenKind::PipePipeEq
          | TokenKind::CaretEq
      );
      if is_assign {
        p.advance();
        let op = match kind {
          TokenKind::Eq => AssignmentOp::Assign,
          TokenKind::PlusEq => AssignmentOp::AddAssign,
          TokenKind::MinusEq => AssignmentOp::SubAssign,
          TokenKind::StarEq => AssignmentOp::MulAssign,
          TokenKind::SlashEq => AssignmentOp::DivAssign,
          TokenKind::PercentEq => AssignmentOp::ModAssign,
          TokenKind::AmpEq => AssignmentOp::BitAndAssign,
          TokenKind::PipeEq => AssignmentOp::BitOrAssign,
          TokenKind::QuestionQuestionEq => AssignmentOp::NullishAssign,
          TokenKind::AmpAmpEq => AssignmentOp::AndAssign,
          TokenKind::PipePipeEq => AssignmentOp::OrAssign,
          TokenKind::CaretEq => AssignmentOp::BitXorAssign,
          _ => {
            p.diagnostics.push(Diagnostic::error(
              "E999",
              "Internal parser error: unexpected assignment operator",
              p.peek().span,
            ));
            AssignmentOp::Assign
          }
        };
        let right = parse_expression(p, if is_right { prec } else { prec + 1 })?;
        let span = Span::new(left.span().start, right.span().end);
        left = Box::new(Expression::AssignmentExpression { left, operator: op, right, span });
        continue;
      }
      p.advance();
      let op = binary_op_from_token(&kind);
      let right = parse_expression(p, if is_right { prec } else { prec + 1 })?;
      let span = Span::new(left.span().start, right.span().end);
      left = Box::new(Expression::BinaryExpression { left, operator: op, right, span });
      continue;
    }
    break;
  }
  // Handle `x as Type` type assertion
  if p.peek().kind == TokenKind::As {
    let start = left.span().start;
    p.advance(); // consume 'as'
    let type_ann = p.parse_type().unwrap_or(TypeAnn::Any);
    let end = p.last_end();
    left = Box::new(Expression::AsExpression {
      expression: left,
      type_ann,
      span: Span::new(start, end),
    });
  }
  // Handle `x satisfies Type`
  if p.peek().kind == TokenKind::Satisfies {
    let start = left.span().start;
    p.advance(); // consume 'satisfies'
    let type_ann = p.parse_type().unwrap_or(TypeAnn::Any);
    let end = p.last_end();
    left = Box::new(Expression::SatisfiesExpression {
      expression: left,
      type_ann,
      span: Span::new(start, end),
    });
  }
  Some(left)
}

fn binary_op_from_token(kind: &TokenKind) -> BinaryOp {
  match kind {
    TokenKind::QuestionQuestion => BinaryOp::NullishCoalescing,
    TokenKind::PipePipe => BinaryOp::LogicalOr,
    TokenKind::AmpAmp => BinaryOp::LogicalAnd,
    TokenKind::Pipe => BinaryOp::BitwiseOr,
    TokenKind::Caret => BinaryOp::BitwiseXor,
    TokenKind::Amp => BinaryOp::BitwiseAnd,
    TokenKind::EqEq => BinaryOp::Eq,
    TokenKind::NotEq => BinaryOp::NotEq,
    TokenKind::EqEqEq => BinaryOp::StrictEq,
    TokenKind::NotEqEq => BinaryOp::StrictNotEq,
    TokenKind::Lt => BinaryOp::Lt,
    TokenKind::Gt => BinaryOp::Gt,
    TokenKind::LtEq => BinaryOp::LtEq,
    TokenKind::GtEq => BinaryOp::GtEq,
    TokenKind::LtLt => BinaryOp::Lsh,
    TokenKind::GtGt => BinaryOp::Rsh,
    TokenKind::GtGtGt => BinaryOp::ZeroFillRsh,
    TokenKind::Plus => BinaryOp::Add,
    TokenKind::Minus => BinaryOp::Sub,
    TokenKind::Star => BinaryOp::Mul,
    TokenKind::Slash => BinaryOp::Div,
    TokenKind::Percent => BinaryOp::Rem,
    TokenKind::StarStar => BinaryOp::Exp,
    TokenKind::Instanceof => BinaryOp::Instanceof,
    _ => BinaryOp::Add,
  }
}

fn parse_prefix(p: &mut Parser) -> Option<Box<Expression>> {
  let kind = p.peek().kind.clone();
  match &kind {
    TokenKind::Number(n) => {
      let tok = p.advance();
      Some(Box::new(Expression::NumberLiteral { value: *n, span: tok.span }))
    }
    TokenKind::String(s) => {
      let tok = p.advance();
      Some(Box::new(Expression::StringLiteral { value: s.clone(), span: tok.span }))
    }
    TokenKind::True => {
      let tok = p.advance();
      Some(Box::new(Expression::BooleanLiteral { value: true, span: tok.span }))
    }
    TokenKind::False => {
      let tok = p.advance();
      Some(Box::new(Expression::BooleanLiteral { value: false, span: tok.span }))
    }
    TokenKind::Null => {
      let tok = p.advance();
      Some(Box::new(Expression::NullLiteral { span: tok.span }))
    }
    TokenKind::Undefined => {
      let tok = p.advance();
      Some(Box::new(Expression::UndefinedLiteral { span: tok.span }))
    }
    TokenKind::Identifier(name) => {
      let tok = p.advance();
      let span = tok.span;
      let name = name.clone();
      // Single-param arrow: x => ...
      if p.peek().kind == TokenKind::Arrow {
        let params =
          vec![Parameter { name, type_ann: None, default_value: None, is_rest: false, span }];
        p.advance(); // =>
        let (body, end) = parse_arrow_body(p);
        return Some(Box::new(Expression::ArrowFunction {
          params,
          return_type: None,
          body,
          is_async: false,
          span: Span::new(span.start, end),
        }));
      }
      Some(Box::new(Expression::Identifier { name, span }))
    }
    TokenKind::OpenParen => {
      let open = p.advance().span;
      // Check if arrow function: (params) => ... or (params): RetType => ...
      if is_arrow_after_paren(&p.tokens, p.cursor - 1) {
        let params = p.parse_function_params();
        p.expect(TokenKind::CloseParen);
        // Parse optional return type: ): Type =>
        let return_type =
          if p.peek().kind == TokenKind::Colon { p.parse_type_annotation() } else { None };
        p.expect(TokenKind::Arrow);
        let (body, body_end) = parse_arrow_body(p);
        return Some(Box::new(Expression::ArrowFunction {
          params,
          return_type,
          body,
          is_async: false,
          span: Span::new(open.start, body_end),
        }));
      }
      let expr = parse_expression(p, 0)?;
      p.expect(TokenKind::CloseParen);
      let end = p.last_end();
      Some(Box::new(Expression::ParenthesizedExpression {
        expression: expr,
        span: Span::new(open.start, end),
      }))
    }
    TokenKind::DotDotDot => {
      let start = p.advance().span.start;
      let argument = parse_expression(p, 1)?;
      let span = Span::new(start, argument.span().end);
      Some(Box::new(Expression::SpreadElement { argument, span }))
    }
    TokenKind::OpenBracket => {
      let open = p.advance().span;
      let mut elements = Vec::new();
      while p.peek().kind != TokenKind::CloseBracket && !p.is_at_end() {
        if p.peek().kind == TokenKind::Comma {
          elements.push(None);
          p.advance();
        } else if p.peek().kind == TokenKind::CloseBracket {
          break;
        } else {
          elements.push(Some(parse_expression(p, 0)?));
          if p.peek().kind == TokenKind::Comma {
            p.advance();
          }
        }
      }
      p.expect(TokenKind::CloseBracket);
      let end = p.last_end();
      Some(Box::new(Expression::ArrayExpression { elements, span: Span::new(open.start, end) }))
    }
    TokenKind::OpenBrace => {
      let open = p.advance().span;
      let mut properties = Vec::new();
      while p.peek().kind != TokenKind::CloseBrace && !p.is_at_end() {
        let prop_start = p.peek().span;
        // Spread element: { ...expr }
        if p.peek().kind == TokenKind::DotDotDot {
          p.advance(); // consume ...
          let argument = parse_expression(p, 0)?;
          let end = argument.span().end;
          properties.push(Property {
            key: PropertyKey::Identifier(String::new()),
            value: argument,
            shorthand: false,
            is_spread: true,
            span: Span::new(prop_start.start, end),
          });
          if p.peek().kind == TokenKind::Comma {
            p.advance();
          }
          continue;
        }
        let computed = p.peek().kind == TokenKind::OpenBracket;
        if computed {
          p.advance();
          let key_expr = parse_expression(p, 0)?;
          p.expect(TokenKind::CloseBracket);
          if p.peek().kind == TokenKind::Colon {
            p.advance();
            let value = parse_expression(p, 0)?;
            let end = value.span().end;
            properties.push(Property {
              key: PropertyKey::Expression(key_expr),
              value,
              shorthand: false,
              is_spread: false,
              span: Span::new(prop_start.start, end),
            });
          }
        } else {
          let kind = p.peek().kind.clone();
          if let TokenKind::Identifier(name) = &kind {
            p.advance();
            if p.peek().kind == TokenKind::Colon {
              p.advance();
              let value = parse_expression(p, 0)?;
              let end = value.span().end;
              properties.push(Property {
                key: PropertyKey::Identifier(name.clone()),
                value,
                shorthand: false,
                is_spread: false,
                span: Span::new(prop_start.start, end),
              });
            } else {
              properties.push(Property {
                key: PropertyKey::Identifier(name.clone()),
                value: Box::new(Expression::Identifier { name: name.clone(), span: prop_start }),
                shorthand: true,
                is_spread: false,
                span: prop_start,
              });
            }
          } else if let TokenKind::String(name) = &kind {
            p.advance();
            if p.peek().kind == TokenKind::Colon {
              p.advance();
              let value = parse_expression(p, 0)?;
              let end = value.span().end;
              properties.push(Property {
                key: PropertyKey::String(name.clone()),
                value,
                shorthand: false,
                is_spread: false,
                span: Span::new(prop_start.start, end),
              });
            } else {
              properties.push(Property {
                key: PropertyKey::String(name.clone()),
                value: Box::new(Expression::Identifier { name: name.clone(), span: prop_start }),
                shorthand: false,
                is_spread: false,
                span: prop_start,
              });
            }
          } else if kind.is_reserved_for_property_key() {
            // Keyword as property key — treat as identifier
            let kw_str = format!("{}", kind);
            p.advance();
            if p.peek().kind == TokenKind::Colon {
              p.advance();
              let value = parse_expression(p, 0)?;
              let end = value.span().end;
              properties.push(Property {
                key: PropertyKey::Identifier(kw_str),
                value,
                shorthand: false,
                is_spread: false,
                span: Span::new(prop_start.start, end),
              });
            } else {
              properties.push(Property {
                key: PropertyKey::Identifier(kw_str.clone()),
                value: Box::new(Expression::Identifier { name: kw_str, span: prop_start }),
                shorthand: false,
                is_spread: false,
                span: prop_start,
              });
            }
          } else {
            p.advance();
            continue;
          }
        }
        if p.peek().kind == TokenKind::Comma {
          p.advance();
        }
      }
      p.expect(TokenKind::CloseBrace);
      let end = p.last_end();
      Some(Box::new(Expression::ObjectExpression { properties, span: Span::new(open.start, end) }))
    }
    TokenKind::NoSubstitutionTemplate(text) => {
      let tok = p.advance();
      Some(Box::new(Expression::TemplateLiteral {
        quasis: vec![text.clone()],
        expressions: vec![],
        span: tok.span,
      }))
    }
    TokenKind::TemplateHead(text) => {
      let start = p.peek().span;
      let mut quasis = vec![text.clone()];
      let mut expressions = Vec::new();
      p.advance(); // consume TemplateHead
      if let Some(expr) = parse_expression(p, 0) {
        expressions.push(expr);
      }
      loop {
        match &p.peek().kind {
          TokenKind::TemplateMiddle(t) => {
            quasis.push(t.clone());
            p.advance();
            if let Some(expr) = parse_expression(p, 0) {
              expressions.push(expr);
            }
          }
          TokenKind::TemplateTail(t) => {
            quasis.push(t.clone());
            let tok = p.advance();
            return Some(Box::new(Expression::TemplateLiteral {
              quasis,
              expressions,
              span: Span::new(start.start, tok.span.end),
            }));
          }
          _ => {
            return Some(Box::new(Expression::TemplateLiteral {
              quasis,
              expressions,
              span: Span::new(start.start, p.peek().span.start),
            }));
          }
        }
      }
    }
    TokenKind::Plus
    | TokenKind::Minus
    | TokenKind::Bang
    | TokenKind::Tilde
    | TokenKind::PlusPlus
    | TokenKind::MinusMinus
    | TokenKind::TypeOf
    | TokenKind::Void
    | TokenKind::Delete => {
      let tok = p.advance().kind.clone();
      let op = match tok {
        TokenKind::Plus => UnaryOp::Plus,
        TokenKind::Minus => UnaryOp::Minus,
        TokenKind::Bang => UnaryOp::Not,
        TokenKind::Tilde => UnaryOp::BitwiseNot,
        TokenKind::PlusPlus => UnaryOp::PlusPlus,
        TokenKind::MinusMinus => UnaryOp::MinusMinus,
        TokenKind::TypeOf => UnaryOp::TypeOf,
        TokenKind::Void => UnaryOp::Void,
        TokenKind::Delete => UnaryOp::Delete,
        _ => UnaryOp::Plus,
      };
      let operand = parse_expression(p, 15)?;
      let start = p.last_start();
      let span = Span::new(start, operand.span().end);
      Some(Box::new(Expression::UnaryExpression { operator: op, operand, span }))
    }
    TokenKind::This => {
      let tok = p.advance();
      Some(Box::new(Expression::ThisExpression { span: tok.span }))
    }
    TokenKind::Super => {
      let tok = p.advance();
      Some(Box::new(Expression::SuperExpression { span: tok.span }))
    }
    TokenKind::Async => {
      let start = p.advance().span.start;
      // async () => ... or async (x) => ... or async x => ...
      match &p.peek().kind {
        TokenKind::OpenParen => {
          let open = p.advance().span;
          if is_arrow_after_paren(&p.tokens, p.cursor - 1) {
            let params = p.parse_function_params();
            p.expect(TokenKind::CloseParen);
            let return_type =
              if p.peek().kind == TokenKind::Colon { p.parse_type_annotation() } else { None };
            p.expect(TokenKind::Arrow);
            let (body, body_end) = parse_arrow_body(p);
            Some(Box::new(Expression::ArrowFunction {
              params,
              return_type,
              body,
              is_async: true,
              span: Span::new(start, body_end),
            }))
          } else {
            let expr = parse_expression(p, 0)?;
            p.expect(TokenKind::CloseParen);
            let end = p.last_end();
            // Treat as call-like: just a grouped expr after async — shouldn't happen
            Some(Box::new(Expression::ParenthesizedExpression {
              expression: expr,
              span: Span::new(open.start, end),
            }))
          }
        }
        TokenKind::Identifier(name) => {
          let name = name.clone();
          let tok = p.advance();
          let span = tok.span;
          if p.peek().kind == TokenKind::Arrow {
            let params =
              vec![Parameter { name, type_ann: None, default_value: None, is_rest: false, span }];
            p.advance();
            let (body, body_end) = parse_arrow_body(p);
            return Some(Box::new(Expression::ArrowFunction {
              params,
              return_type: None,
              body,
              is_async: true,
              span: Span::new(start, body_end),
            }));
          }
          Some(Box::new(Expression::Identifier { name, span }))
        }
        _ => None,
      }
    }
    TokenKind::Await => {
      let start = p.advance().span.start;
      let argument = parse_expression(p, 15)?;
      let span = Span::new(start, argument.span().end);
      Some(Box::new(Expression::AwaitExpression { argument, span }))
    }
    TokenKind::Import => {
      let start = p.advance().span.start;
      if p.peek().kind == TokenKind::Dot {
        p.advance(); // consume '.'
        // Check for 'meta'
        if let TokenKind::Identifier(n) = &p.peek().kind {
          if n == "meta" {
            let meta_span = p.advance().span;
            let end = meta_span.end;
            return Some(Box::new(Expression::ImportMeta { span: Span::new(start, end) }));
          }
        }
      }
      // import("source") dynamic import
      if p.peek().kind == TokenKind::OpenParen {
        p.advance(); // consume (
        let source = parse_expression(p, 0)
          .unwrap_or_else(|| Box::new(Expression::Placeholder { span: p.peek().span }));
        p.expect(TokenKind::CloseParen);
        let end = p.last_end();
        return Some(Box::new(Expression::ImportCall { source, span: Span::new(start, end) }));
      }
      None
    }
    TokenKind::New => {
      let start = p.advance().span.start;
      // Parse callee — could be dotted: new Error, new foo.Bar
      let callee = match &p.peek().kind {
        TokenKind::Identifier(name) => {
          let name = name.clone();
          let tok = p.advance();
          let mut n = Box::new(Expression::Identifier { name, span: tok.span });
          while p.peek().kind == TokenKind::Dot {
            p.advance();
            if let TokenKind::Identifier(prop) = &p.peek().kind {
              let prop_name = prop.clone();
              let prop_span = p.advance().span;
              let prev_span = n.span();
              n = Box::new(Expression::MemberExpression {
                object: n,
                property: Box::new(Expression::Identifier { name: prop_name, span: prop_span }),
                computed: false,
                optional: false,
                span: Span::new(prev_span.start, prop_span.end),
              });
            }
          }
          n
        }
        _ => {
          return None;
        }
      };
      // Optionally parse (args)
      let args = if p.peek().kind == TokenKind::OpenParen {
        p.advance();
        parse_arguments(p)
      } else {
        Vec::new()
      };
      let end = if args.is_empty() { callee.span().end } else { p.last_end() };
      Some(Box::new(Expression::NewExpression {
        callee,
        arguments: args,
        span: Span::new(start, end),
      }))
    }
    // JSX: <Tag ...> or <Tag ... />
    // At prefix position < is always JSX (no left operand for comparison)
    TokenKind::Lt if matches!(p.peek_ahead(1).kind, TokenKind::Identifier(_)) => {
      Some(parse_jsx_element(p))
    }
    _ => None,
  }
}

#[allow(clippy::vec_box)]
fn parse_arguments(p: &mut Parser) -> Vec<Box<Expression>> {
  let mut args = Vec::new();
  while p.peek().kind != TokenKind::CloseParen && !p.is_at_end() {
    if let Some(arg) = parse_expression(p, 0) {
      args.push(arg);
    }
    if p.peek().kind == TokenKind::Comma {
      p.advance();
    } else {
      break;
    }
  }
  p.expect(TokenKind::CloseParen);
  args
}

fn is_arrow_after_paren(tokens: &[Token], paren_idx: usize) -> bool {
  let mut depth = 0u32;
  let mut close_idx = None;
  for (i, tok) in tokens.iter().enumerate().skip(paren_idx) {
    match &tok.kind {
      TokenKind::OpenParen => depth += 1,
      TokenKind::CloseParen => {
        depth -= 1;
        if depth == 0 {
          close_idx = Some(i);
          break;
        }
      }
      _ => {}
    }
  }
  let Some(ci) = close_idx else { return false };
  // ) => ...
  if ci + 1 < tokens.len() && tokens[ci + 1].kind == TokenKind::Arrow {
    return true;
  }
  // ): Type => ...  — skip past colon + type to find =>
  if ci + 1 < tokens.len() && tokens[ci + 1].kind == TokenKind::Colon {
    // Scan forward: after `:`, skip tokens until we find `=>`
    for tok in tokens.iter().skip(ci + 2) {
      if tok.kind == TokenKind::Arrow {
        return true;
      }
      // If we hit a statement boundary or other keyword, bail
      if matches!(
        tok.kind,
        TokenKind::Semicolon
          | TokenKind::OpenBrace
          | TokenKind::Return
          | TokenKind::If
          | TokenKind::While
          | TokenKind::For
      ) {
        return false;
      }
    }
  }
  false
}

fn parse_arrow_body(p: &mut Parser) -> (ArrowFunctionBody, usize) {
  if p.peek().kind == TokenKind::OpenBrace {
    let block = p.parse_block();
    match block {
      Statement::BlockStatement { body, span, .. } => (ArrowFunctionBody::Block(body), span.end),
      _ => {
        let span = p.peek().span;
        (ArrowFunctionBody::Expression(Box::new(Expression::Placeholder { span })), span.end)
      }
    }
  } else if let Some(expr) = parse_expression(p, 1) {
    let end = expr.span().end;
    (ArrowFunctionBody::Expression(expr), end)
  } else {
    // Fallback: empty body
    let span = p.peek().span;
    (ArrowFunctionBody::Expression(Box::new(Expression::Placeholder { span })), span.end)
  }
}

fn parse_jsx_element(p: &mut Parser) -> Box<Expression> {
  use crate::ast::{JsxAttribute, JsxAttributeValue};
  let start = p.peek().span.start;
  p.advance(); // consume <
  // Tag name
  let tag = if let TokenKind::Identifier(n) = &p.peek().kind {
    let n = n.clone();
    let span = p.advance().span;
    Box::new(Expression::Identifier { name: n, span })
  } else {
    return Box::new(Expression::Placeholder { span: p.peek().span });
  };

  let mut attributes = Vec::new();
  let mut children = Vec::new();
  let mut self_closing = false;

  // Parse attributes until > or />
  loop {
    match &p.peek().kind {
      TokenKind::Gt => {
        p.advance();
        break;
      }
      TokenKind::Slash if matches!(p.peek_ahead(1).kind, TokenKind::Gt) => {
        p.advance(); // /
        p.advance(); // >
        self_closing = true;
        break;
      }
      TokenKind::Identifier(name) => {
        let attr_name = name.clone();
        let attr_start = p.peek().span.start;
        p.advance();
        if p.peek().kind == TokenKind::Eq {
          p.advance(); // =
          let value = if let TokenKind::String(s) = &p.peek().kind {
            let s = s.clone();
            p.advance();
            JsxAttributeValue::Text(s)
          } else if p.peek().kind == TokenKind::OpenBrace {
            p.advance(); // {
            let expr = p
              .parse_expression(0)
              .unwrap_or_else(|| Box::new(Expression::Placeholder { span: p.peek().span }));
            if p.peek().kind == TokenKind::CloseBrace {
              p.advance(); // }
            }
            JsxAttributeValue::Expression(expr)
          } else {
            JsxAttributeValue::Text(String::new())
          };
          attributes.push(JsxAttribute {
            name: attr_name,
            value,
            span: Span::new(attr_start, p.last_end()),
          });
        } else {
          // Boolean attribute: <Comp disabled />
          attributes.push(JsxAttribute {
            name: attr_name,
            value: JsxAttributeValue::Expression(Box::new(Expression::BooleanLiteral {
              value: true,
              span: p.peek().span,
            })),
            span: Span::new(attr_start, p.last_end()),
          });
        }
      }
      _ => {
        // Skip unexpected tokens (e.g., whitespace artifacts)
        p.advance();
        continue;
      }
    }
  }

  // Parse children until closing tag
  if !self_closing {
    loop {
      match &p.peek().kind {
        TokenKind::Lt if matches!(p.peek_ahead(1).kind, TokenKind::Slash) => {
          // Closing tag: </Tag>
          p.advance(); // <
          p.advance(); // /
          // Skip tag name identifier
          if matches!(p.peek().kind, TokenKind::Identifier(_)) {
            p.advance();
          }
          p.advance(); // >
          break;
        }
        TokenKind::Eof => break,
        _ => {
          if let Some(child) = parse_jsx_child(p) {
            children.push(child);
          } else {
            break;
          }
        }
      }
    }
  }

  let end = p.last_end();
  Box::new(Expression::JsxElement {
    tag,
    attributes,
    children,
    self_closing,
    span: Span::new(start, end),
  })
}

fn parse_jsx_child(p: &mut Parser) -> Option<crate::ast::JsxChild> {
  use crate::ast::JsxChild;
  match &p.peek().kind {
    // Nested JSX element
    TokenKind::Lt if matches!(p.peek_ahead(1).kind, TokenKind::Identifier(_)) => {
      Some(JsxChild::Element(parse_jsx_element(p)))
    }
    // Expression child: {expr}
    TokenKind::OpenBrace => {
      p.advance(); // {
      let expr = parse_expression(p, 0)?;
      if p.peek().kind == TokenKind::CloseBrace {
        p.advance(); // }
      }
      Some(JsxChild::Expression(expr))
    }
    // Text content — extract raw source between current pos and next < or {
    _ => {
      let text_start = p.peek().span.start;
      while !matches!(p.peek().kind, TokenKind::Lt | TokenKind::OpenBrace | TokenKind::Eof) {
        p.advance();
      }
      let text_end = p.last_end();
      if text_start >= text_end {
        return None;
      }
      // Extract from source to preserve whitespace between tokens
      let text = p.source_file.source[text_start..text_end].trim().to_string();
      if text.is_empty() {
        return None;
      }
      Some(JsxChild::Text(text))
    }
  }
}
