use crate::ast::{ArrowFunctionBody, AssignmentOp, BinaryOp, Expression, PropertyKey, UnaryOp};
use std::fmt::Write;

pub fn binary_precedence(op: BinaryOp) -> u8 {
  match op {
    BinaryOp::NullishCoalescing => 1,
    BinaryOp::LogicalOr => 2,
    BinaryOp::LogicalAnd => 3,
    BinaryOp::BitwiseOr => 4,
    BinaryOp::BitwiseXor => 5,
    BinaryOp::BitwiseAnd => 6,
    BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::StrictEq | BinaryOp::StrictNotEq => 7,
    BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => 8,
    BinaryOp::Lsh | BinaryOp::Rsh | BinaryOp::ZeroFillRsh => 9,
    BinaryOp::Add | BinaryOp::Sub => 10,
    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => 11,
    BinaryOp::Exp => 12,
    BinaryOp::Instanceof => 7,
  }
}

fn op_str(op: BinaryOp) -> &'static str {
  match op {
    BinaryOp::NullishCoalescing => "??",
    BinaryOp::LogicalOr => "||",
    BinaryOp::LogicalAnd => "&&",
    BinaryOp::BitwiseOr => "|",
    BinaryOp::BitwiseXor => "^",
    BinaryOp::BitwiseAnd => "&",
    BinaryOp::Eq => "==",
    BinaryOp::NotEq => "!=",
    BinaryOp::StrictEq => "===",
    BinaryOp::StrictNotEq => "!==",
    BinaryOp::Lt => "<",
    BinaryOp::Gt => ">",
    BinaryOp::LtEq => "<=",
    BinaryOp::GtEq => ">=",
    BinaryOp::Lsh => "<<",
    BinaryOp::Rsh => ">>",
    BinaryOp::ZeroFillRsh => ">>>",
    BinaryOp::Add => "+",
    BinaryOp::Sub => "-",
    BinaryOp::Mul => "*",
    BinaryOp::Div => "/",
    BinaryOp::Rem => "%",
    BinaryOp::Exp => "**",
    BinaryOp::Instanceof => "instanceof",
  }
}

fn unary_op_str(op: UnaryOp) -> &'static str {
  match op {
    UnaryOp::Plus => "+",
    UnaryOp::Minus => "-",
    UnaryOp::Not => "!",
    UnaryOp::BitwiseNot => "~",
    UnaryOp::PlusPlus => "++",
    UnaryOp::MinusMinus => "--",
    UnaryOp::TypeOf => "typeof ",
    UnaryOp::Void => "void ",
    UnaryOp::Delete => "delete ",
  }
}

fn assign_op_str(op: AssignmentOp) -> &'static str {
  match op {
    AssignmentOp::Assign => "=",
    AssignmentOp::AddAssign => "+=",
    AssignmentOp::SubAssign => "-=",
    AssignmentOp::MulAssign => "*=",
    AssignmentOp::DivAssign => "/=",
    AssignmentOp::ModAssign => "%=",
    AssignmentOp::BitAndAssign => "&=",
    AssignmentOp::BitOrAssign => "|=",
    AssignmentOp::BitXorAssign => "^=",
    AssignmentOp::NullishAssign => "??=",
    AssignmentOp::AndAssign => "&&=",
    AssignmentOp::OrAssign => "||=",
  }
}

pub fn gen_expression(cg: &mut super::Codegen, expr: &Expression) {
  match expr {
    Expression::NumberLiteral { value, .. } => {
      if *value == value.floor() && value.is_finite() && value.abs() < 1.0e15 {
        let _ = write!(cg.output, "{}", *value as i64);
      } else {
        let _ = write!(cg.output, "{value}");
      }
    }
    Expression::StringLiteral { value, .. } => {
      let _ = write!(cg.output, "\"{value}\"");
    }
    Expression::BooleanLiteral { value, .. } => {
      let _ = write!(cg.output, "{value}");
    }
    Expression::NullLiteral { .. } => {
      let _ = write!(cg.output, "null");
    }
    Expression::UndefinedLiteral { .. } => {
      let _ = write!(cg.output, "undefined");
    }
    Expression::Identifier { name, .. } => {
      let _ = write!(cg.output, "{name}");
    }
    Expression::ArrayExpression { elements, .. } => {
      let _ = write!(cg.output, "[");
      for (i, elem) in elements.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        if let Some(e) = elem {
          gen_expression(cg, e);
        }
      }
      let _ = write!(cg.output, "]");
    }
    Expression::ObjectExpression { properties, .. } => {
      let _ = write!(cg.output, "{{");
      for (i, prop) in properties.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        if prop.is_spread {
          let _ = write!(cg.output, "...");
          gen_expression(cg, &prop.value);
          continue;
        }
        match &prop.key {
          PropertyKey::Identifier(name) => {
            if prop.shorthand {
              let _ = write!(cg.output, "{name}");
            } else {
              let _ = write!(cg.output, "{name}: ");
            }
          }
          PropertyKey::String(name) => {
            let _ = write!(cg.output, "\"{name}\": ");
          }
          PropertyKey::Expression(key_expr) => {
            let _ = write!(cg.output, "[");
            gen_expression(cg, key_expr);
            let _ = write!(cg.output, "]: ");
          }
        }
        if !prop.shorthand {
          gen_expression(cg, &prop.value);
        }
      }
      let _ = write!(cg.output, "}}");
    }
    Expression::BinaryExpression { left, operator, right, .. } => {
      let needs_l = super::needs_parens(expr, left);
      let needs_r = super::needs_parens(expr, right);
      if needs_l {
        let _ = write!(cg.output, "(");
      }
      gen_expression(cg, left);
      if needs_l {
        let _ = write!(cg.output, ")");
      }
      let _ = write!(cg.output, " {} ", op_str(*operator));
      if needs_r {
        let _ = write!(cg.output, "(");
      }
      gen_expression(cg, right);
      if needs_r {
        let _ = write!(cg.output, ")");
      }
    }
    Expression::UnaryExpression { operator, operand, .. } => {
      if matches!(operator, UnaryOp::PlusPlus | UnaryOp::MinusMinus) {
        gen_expression(cg, operand);
        let _ = write!(cg.output, "{}", unary_op_str(*operator));
      } else {
        let _ = write!(cg.output, "{}", unary_op_str(*operator));
        gen_expression(cg, operand);
      }
    }
    Expression::ConditionalExpression { test, consequent, alternate, .. } => {
      gen_expression(cg, test);
      let _ = write!(cg.output, " ? ");
      gen_expression(cg, consequent);
      let _ = write!(cg.output, " : ");
      gen_expression(cg, alternate);
    }
    Expression::CallExpression { callee, arguments, optional, .. } => {
      gen_expression(cg, callee);
      if *optional {
        let _ = write!(cg.output, "?.(");
      } else {
        let _ = write!(cg.output, "(");
      }
      for (i, arg) in arguments.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        gen_expression(cg, arg);
      }
      let _ = write!(cg.output, ")");
    }
    Expression::MemberExpression { object, property, computed, optional, .. } => {
      gen_expression(cg, object);
      if *computed {
        if *optional {
          let _ = write!(cg.output, "?.[");
        } else {
          let _ = write!(cg.output, "[");
        }
        gen_expression(cg, property);
        let _ = write!(cg.output, "]");
      } else {
        if *optional {
          let _ = write!(cg.output, "?.");
        } else {
          let _ = write!(cg.output, ".");
        }
        gen_expression(cg, property);
      }
    }
    Expression::AssignmentExpression { left, operator, right, .. } => {
      gen_expression(cg, left);
      let _ = write!(cg.output, " {} ", assign_op_str(*operator));
      gen_expression(cg, right);
    }
    Expression::ParenthesizedExpression { expression, .. } => {
      let _ = write!(cg.output, "(");
      gen_expression(cg, expression);
      let _ = write!(cg.output, ")");
    }
    Expression::ArrowFunction { params, body, is_async, .. } => {
      if *is_async {
        let _ = write!(cg.output, "async ");
      }
      if params.len() == 1
        && params[0].type_ann.is_none()
        && params[0].default_value.is_none()
        && !params[0].is_rest
      {
        let _ = write!(cg.output, "{}", params[0].name);
      } else {
        let _ = write!(cg.output, "(");
        for (i, param) in params.iter().enumerate() {
          if i > 0 {
            let _ = write!(cg.output, ", ");
          }
          if param.is_rest {
            let _ = write!(cg.output, "...");
          }
          let _ = write!(cg.output, "{}", param.name);
          if let Some(default) = &param.default_value {
            let _ = write!(cg.output, " = ");
            gen_expression(cg, default);
          }
        }
        let _ = write!(cg.output, ")");
      }
      let _ = write!(cg.output, " => ");
      match body {
        ArrowFunctionBody::Expression(expr) => gen_expression(cg, expr),
        ArrowFunctionBody::Block(stmts) => {
          let _ = write!(cg.output, "{{");
          let saved = cg.indent;
          cg.indent += 1;
          let _ = writeln!(cg.output);
          for stmt in stmts {
            super::Codegen::gen_statement(cg, stmt);
          }
          cg.indent = saved;
          let _ = write!(cg.output, "}}");
        }
      }
    }
    Expression::Placeholder { .. } => {}
    Expression::TemplateLiteral { quasis, expressions, .. } => {
      let _ = write!(cg.output, "`");
      for (i, quasi) in quasis.iter().enumerate() {
        let _ = write!(cg.output, "{quasi}");
        if i < expressions.len() {
          let _ = write!(cg.output, "${{");
          gen_expression(cg, &expressions[i]);
          let _ = write!(cg.output, "}}");
        }
      }
      let _ = write!(cg.output, "`");
    }
    Expression::NewExpression { callee, arguments, .. } => {
      let _ = write!(cg.output, "new ");
      gen_expression(cg, callee);
      let _ = write!(cg.output, "(");
      for (i, arg) in arguments.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        gen_expression(cg, arg);
      }
      let _ = write!(cg.output, ")");
    }
    Expression::SpreadElement { argument, .. } => {
      let _ = write!(cg.output, "...");
      gen_expression(cg, argument);
    }
    Expression::ThisExpression { .. } => {
      let _ = write!(cg.output, "this");
    }
    Expression::SuperExpression { .. } => {
      let _ = write!(cg.output, "super");
    }
    Expression::ObjectPattern { properties, .. } => {
      let _ = write!(cg.output, "{{");
      for (i, prop) in properties.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        match &prop.key {
          PropertyKey::Identifier(name) => {
            if prop.shorthand {
              let _ = write!(cg.output, "{name}");
            } else {
              let _ = write!(cg.output, "{name}: ");
              gen_expression(cg, &prop.value);
            }
          }
          PropertyKey::String(name) => {
            let _ = write!(cg.output, "\"{name}\": ");
            gen_expression(cg, &prop.value);
          }
          PropertyKey::Expression(key_expr) => {
            let _ = write!(cg.output, "[");
            gen_expression(cg, key_expr);
            let _ = write!(cg.output, "]: ");
            gen_expression(cg, &prop.value);
          }
        }
      }
      let _ = write!(cg.output, "}}");
    }
    Expression::ArrayPattern { elements, .. } => {
      let _ = write!(cg.output, "[");
      for (i, elem) in elements.iter().enumerate() {
        if i > 0 {
          let _ = write!(cg.output, ", ");
        }
        if let Some(e) = elem {
          gen_expression(cg, e);
        }
      }
      let _ = write!(cg.output, "]");
    }
    Expression::AwaitExpression { argument, .. } => {
      let _ = write!(cg.output, "await ");
      gen_expression(cg, argument);
    }
    Expression::AsExpression { expression, .. } => {
      gen_expression(cg, expression);
    }
    Expression::SatisfiesExpression { expression, .. } => {
      gen_expression(cg, expression);
    }
    Expression::TaggedTemplateLiteral { tag, quasis, expressions, .. } => {
      gen_expression(cg, tag);
      let _ = write!(cg.output, "`");
      for (i, quasi) in quasis.iter().enumerate() {
        let _ = write!(cg.output, "{quasi}");
        if let Some(expr) = expressions.get(i) {
          let _ = write!(cg.output, "${{");
          gen_expression(cg, expr);
          let _ = write!(cg.output, "}}");
        }
      }
      let _ = write!(cg.output, "`");
    }
    Expression::ImportMeta { .. } => {
      let _ = write!(cg.output, "import.meta");
    }
    Expression::ImportCall { source, .. } => {
      let _ = write!(cg.output, "import(");
      gen_expression(cg, source);
      let _ = write!(cg.output, ")");
    }
    Expression::JsxElement { tag, attributes, children, .. } => {
      let _ = write!(cg.output, "React.createElement(");
      gen_expression(cg, tag);
      // Props object
      if attributes.is_empty() {
        let _ = write!(cg.output, ", null");
      } else {
        let _ = write!(cg.output, ", {{");
        for (i, attr) in attributes.iter().enumerate() {
          if i > 0 {
            let _ = write!(cg.output, ", ");
          }
          let _ = write!(cg.output, "{}: ", attr.name);
          match &attr.value {
            crate::ast::JsxAttributeValue::Text(s) => {
              let _ = write!(cg.output, "\"{s}\"");
            }
            crate::ast::JsxAttributeValue::Expression(e) => {
              gen_expression(cg, e);
            }
          }
        }
        let _ = write!(cg.output, "}}");
      }
      // Children
      for child in children {
        let _ = write!(cg.output, ", ");
        match child {
          crate::ast::JsxChild::Text(s) => {
            let _ = write!(cg.output, "\"{s}\"");
          }
          crate::ast::JsxChild::Expression(e) => {
            gen_expression(cg, e);
          }
          crate::ast::JsxChild::Element(e) => {
            gen_expression(cg, e);
          }
        }
      }
      let _ = write!(cg.output, ")");
    }
  }
}
