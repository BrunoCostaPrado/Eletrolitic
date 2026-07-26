use crate::ast::{Expression, Program, Statement};

pub struct TreeShaker;

impl TreeShaker {
  pub fn shake(program: &Program) -> Program {
    Program { body: program.body.iter().filter_map(Self::shake_stmt).collect() }
  }

  fn shake_stmt(stmt: &Statement) -> Option<Statement> {
    match stmt {
      Statement::IfStatement { test, consequent, alternate, span } => {
        if Self::is_false_literal(test) {
          // if (false) { ... } → drop entire statement
          return alternate.as_ref().and_then(|alt| Self::shake_stmt(alt));
        }
        Some(Statement::IfStatement {
          test: test.clone(),
          consequent: Box::new(
            Self::shake_stmt(consequent)
              .unwrap_or_else(|| Statement::BlockStatement { body: vec![], span: *span }),
          ),
          alternate: alternate.as_ref().and_then(|alt| Self::shake_stmt(alt).map(Box::new)),
          span: *span,
        })
      }
      Statement::BlockStatement { body, span } => Some(Statement::BlockStatement {
        body: body.iter().filter_map(Self::shake_stmt).collect(),
        span: *span,
      }),
      Statement::ExportDeclaration { declaration, is_default, span } => {
        Self::shake_stmt(declaration).map(|decl| Statement::ExportDeclaration {
          declaration: Box::new(decl),
          is_default: *is_default,
          span: *span,
        })
      }
      Statement::FunctionDeclaration {
        body,
        name,
        params,
        return_type,
        is_async,
        type_params,
        ..
      } => {
        if let Statement::BlockStatement { body: fn_body, span } = body.as_ref() {
          let shaken: Vec<_> = fn_body.iter().filter_map(Self::shake_stmt).collect();
          if shaken == *fn_body {
            return Some(stmt.clone());
          }
          Some(Statement::FunctionDeclaration {
            name: name.clone(),
            params: params.clone(),
            return_type: return_type.clone(),
            body: Box::new(Statement::BlockStatement { body: shaken, span: *span }),
            is_async: *is_async,
            type_params: type_params.clone(),
            span: *span,
          })
        } else {
          Some(stmt.clone())
        }
      }
      _ => Some(stmt.clone()),
    }
  }

  fn is_false_literal(expr: &Expression) -> bool {
    matches!(expr, Expression::BooleanLiteral { value: false, .. })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::VariableKind;
  use crate::token::Span;

  const SP: Span = Span { start: 0, end: 0 };

  fn expr_stmt(expr: Expression) -> Statement {
    Statement::ExpressionStatement { expression: Box::new(expr), span: SP }
  }

  fn ident(name: &str) -> Expression {
    Expression::Identifier { name: name.to_string(), span: SP }
  }

  fn bool_expr(value: bool) -> Expression {
    Expression::BooleanLiteral { value, span: SP }
  }

  fn var_decl(name: &str) -> Statement {
    Statement::VariableDeclaration {
      kind: VariableKind::Const,
      declarations: vec![crate::ast::VariableDeclarator {
        id: Box::new(ident(name)),
        type_ann: None,
        init: Some(Box::new(Expression::NumberLiteral { value: 1.0, span: SP })),
        span: SP,
      }],
      span: SP,
    }
  }

  fn if_false(consequent: Statement) -> Statement {
    Statement::IfStatement {
      test: Box::new(bool_expr(false)),
      consequent: Box::new(consequent),
      alternate: None,
      span: SP,
    }
  }

  fn if_false_else(consequent: Statement, alt: Statement) -> Statement {
    Statement::IfStatement {
      test: Box::new(bool_expr(false)),
      consequent: Box::new(consequent),
      alternate: Some(Box::new(alt)),
      span: SP,
    }
  }

  fn func_decl(name: &str, body: Vec<Statement>) -> Statement {
    Statement::FunctionDeclaration {
      name: name.to_string(),
      params: vec![],
      return_type: None,
      body: Box::new(Statement::BlockStatement { body, span: SP }),
      is_async: false,
      type_params: vec![],
      span: SP,
    }
  }

  fn wrap_export(stmt: Statement) -> Statement {
    Statement::ExportDeclaration { declaration: Box::new(stmt), is_default: false, span: SP }
  }

  fn shake(body: Vec<Statement>) -> Program {
    TreeShaker::shake(&Program { body })
  }

  #[test]
  fn empty_program() {
    let out = shake(vec![]);
    assert!(out.body.is_empty());
  }

  #[test]
  fn passthrough_non_if() {
    let stmt = var_decl("x");
    let out = shake(vec![stmt.clone()]);
    assert_eq!(out.body, vec![stmt]);
  }

  #[test]
  fn drop_if_false_no_else() {
    let out = shake(vec![if_false(expr_stmt(ident("oops")))]);
    assert!(out.body.is_empty());
  }

  #[test]
  fn if_false_else_keeps_alt() {
    let alt = var_decl("kept");
    let out = shake(vec![if_false_else(expr_stmt(ident("dead")), alt.clone())]);
    assert_eq!(out.body, vec![alt]);
  }

  #[test]
  fn nested_block_strips_false_if() {
    let inner = Statement::BlockStatement {
      body: vec![var_decl("a"), if_false(expr_stmt(ident("b"))), var_decl("c")],
      span: SP,
    };
    let out = shake(vec![inner]);
    let block = match &out.body[0] {
      Statement::BlockStatement { body, .. } => body,
      _ => panic!("expected block"),
    };
    assert_eq!(block.len(), 2);
  }

  #[test]
  fn export_preserved_after_shake() {
    let exported = wrap_export(if_false_else(expr_stmt(ident("dead")), var_decl("live")));
    let out = shake(vec![exported]);
    match &out.body[0] {
      Statement::ExportDeclaration { declaration, .. } => match declaration.as_ref() {
        Statement::VariableDeclaration { .. } => {}
        other => panic!("expected var decl inside export, got {other:?}"),
      },
      other => panic!("expected export, got {other:?}"),
    }
  }

  #[test]
  fn func_body_shaken() {
    let f = func_decl("f", vec![var_decl("a"), if_false(expr_stmt(ident("b"))), var_decl("c")]);
    let out = shake(vec![f]);
    match &out.body[0] {
      Statement::FunctionDeclaration { body, .. } => {
        if let Statement::BlockStatement { body: fb, .. } = body.as_ref() {
          assert_eq!(fb.len(), 2);
        } else {
          panic!("expected block body");
        }
      }
      other => panic!("expected func decl, got {other:?}"),
    }
  }

  #[test]
  fn true_if_preserved() {
    let stmt = Statement::IfStatement {
      test: Box::new(bool_expr(true)),
      consequent: Box::new(var_decl("x")),
      alternate: None,
      span: SP,
    };
    let out = shake(vec![stmt]);
    assert_eq!(out.body.len(), 1);
  }

  #[test]
  fn if_false_else_if_true_keeps_both_branches() {
    // if (false) { a } else if (true) { b } → { b }
    let else_if = Statement::IfStatement {
      test: Box::new(bool_expr(true)),
      consequent: Box::new(var_decl("b")),
      alternate: None,
      span: SP,
    };
    let stmt = if_false_else(expr_stmt(ident("a")), else_if);
    let out = shake(vec![stmt]);
    assert_eq!(out.body.len(), 1);
  }
}
