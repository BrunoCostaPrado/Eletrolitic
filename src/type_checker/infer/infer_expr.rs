use crate::ast::{BinaryOp, Expression, LiteralValue, PropertyKey, UnaryOp};
use crate::type_checker::TypeChecker;
use crate::type_checker::env::TypeEnv;
use crate::type_checker::error::TypeError;
use crate::type_checker::ty::Type;

impl TypeChecker {
  pub fn infer_expression(&mut self, expr: &Expression, env: &mut TypeEnv) -> Type {
    match expr {
      Expression::NumberLiteral { value, .. } => Type::Literal(LiteralValue::Number(*value)),
      Expression::StringLiteral { value, .. } => Type::Literal(LiteralValue::String(value.clone())),
      Expression::BooleanLiteral { value, .. } => Type::Literal(LiteralValue::Boolean(*value)),
      Expression::NullLiteral { .. } => Type::Null,
      Expression::UndefinedLiteral { .. } => Type::Undefined,
      Expression::Identifier { name, span, .. } => {
        // Type guard narrowing: check narrowed map first
        if let Some(narrowed) = self.narrowed.get(name) {
          return narrowed.clone();
        }
        env.lookup(name).unwrap_or_else(|| {
          self.errors.push(TypeError::CannotFindName(name.clone(), *span));
          Type::Any
        })
      }
      Expression::BinaryExpression { operator, left, right, .. } => {
        let lt = self.infer_expression(left, env);
        let rt = self.infer_expression(right, env);
        match operator {
          BinaryOp::Add => {
            // ponytail: + returns String if either operand is String
            if matches!(lt, Type::String | Type::Literal(LiteralValue::String(_)))
              || matches!(rt, Type::String | Type::Literal(LiteralValue::String(_)))
            {
              Type::String
            } else {
              Type::Number
            }
          }
          BinaryOp::Sub
          | BinaryOp::Mul
          | BinaryOp::Div
          | BinaryOp::Rem
          | BinaryOp::Exp
          | BinaryOp::BitwiseOr
          | BinaryOp::BitwiseXor
          | BinaryOp::BitwiseAnd
          | BinaryOp::Lsh
          | BinaryOp::Rsh
          | BinaryOp::ZeroFillRsh => Type::Number,
          BinaryOp::Eq
          | BinaryOp::NotEq
          | BinaryOp::StrictEq
          | BinaryOp::StrictNotEq
          | BinaryOp::Lt
          | BinaryOp::Gt
          | BinaryOp::LtEq
          | BinaryOp::GtEq
          | BinaryOp::LogicalOr
          | BinaryOp::LogicalAnd
          | BinaryOp::Instanceof => Type::Boolean,
          BinaryOp::NullishCoalescing => lt,
        }
      }
      Expression::UnaryExpression { operator, operand: _, .. } => match operator {
        UnaryOp::Minus
        | UnaryOp::Plus
        | UnaryOp::BitwiseNot
        | UnaryOp::PlusPlus
        | UnaryOp::MinusMinus => Type::Number,
        UnaryOp::Not => Type::Boolean,
        UnaryOp::TypeOf => Type::Literal(LiteralValue::String("string".into())),
        UnaryOp::Void => Type::Undefined,
        UnaryOp::Delete => Type::Boolean,
      },
      Expression::ConditionalExpression { consequent, alternate, .. } => {
        let ct = self.infer_expression(consequent, env);
        let at = self.infer_expression(alternate, env);
        if ct == at { ct } else { Type::Union(vec![ct, at]) }
      }
      Expression::ArrayExpression { elements, .. } => {
        let mut elem_types = Vec::new();
        for elem in elements.iter().flatten() {
          let t = self.infer_expression(elem, env);
          if !elem_types.contains(&t) {
            elem_types.push(t);
          }
        }
        let element_type = if elem_types.is_empty() {
          Type::Any
        } else if elem_types.len() == 1 {
          elem_types.swap_remove(0)
        } else {
          Type::Union(elem_types)
        };
        Type::Array(Box::new(element_type))
      }
      Expression::ObjectExpression { properties, .. } => {
        let mut fields = Vec::new();
        for prop in properties {
          if prop.is_spread {
            self.infer_expression(&prop.value, env);
            continue;
          }
          let key = match &prop.key {
            PropertyKey::Identifier(name) => name.clone(),
            PropertyKey::String(name) => name.clone(),
            PropertyKey::Expression(_) => continue, // can't infer computed keys statically
          };
          let value_type = self.infer_expression(&prop.value, env);
          fields.push((key, value_type));
        }
        Type::Object { fields }
      }
      Expression::CallExpression { callee, arguments, span, optional, .. } => {
        let callee_type = self.infer_expression(callee, env);
        let result = if let Type::Function { params, return_type } = callee_type {
          // Check argument count
          if params.len() != arguments.len() {
            self.errors.push(TypeError::ArgumentCountMismatch {
              expected: params.len(),
              found: arguments.len(),
              span: *span,
            });
          }
          // ponytail: generic inference — collect TypeParam → actual type mappings
          let mut type_bindings: Vec<(String, Type)> = Vec::new();
          let mut resolved_params = params.clone();
          for (i, arg) in arguments.iter().enumerate() {
            let arg_type = self.infer_expression(arg, env);
            if i < resolved_params.len() {
              // If param is TypeParam, bind it to arg_type
              if let Type::TypeParam(name, _) = &resolved_params[i]
                && !type_bindings.iter().any(|(n, _)| n == name)
              {
                type_bindings.push((name.clone(), arg_type.clone()));
                resolved_params[i] = arg_type.clone();
              }
              if !arg_type.is_assignable_to(&resolved_params[i]) {
                self.errors.push(TypeError::NotAssignable {
                  target: format!("{}", resolved_params[i]),
                  value: format!("{arg_type}"),
                  span: arg.span(),
                });
              }
            }
          }
          // Substitute type bindings in return type
          substitute_type_params(&return_type, &type_bindings)
        } else {
          // Check if callee is a known non-callable name
          if let Expression::Identifier { name, .. } = callee.as_ref()
            && matches!(
              callee_type,
              Type::Number
                | Type::String
                | Type::Boolean
                | Type::Null
                | Type::Undefined
                | Type::Array { .. }
            )
          {
            self.errors.push(TypeError::NotAFunction { name: name.clone(), span: *span });
          }
          for arg in arguments {
            self.infer_expression(arg, env);
          }
          Type::Any
        };
        // ponytail: optional call wraps result in T | undefined
        if *optional {
          if matches!(result, Type::Undefined) {
            result
          } else {
            Type::Union(vec![result, Type::Undefined])
          }
        } else {
          result
        }
      }
      Expression::MemberExpression { object, property, computed, optional, .. } => {
        let obj_type = self.infer_expression(object, env);
        let key = if *computed {
          match property.as_ref() {
            Expression::StringLiteral { value, .. } => Some(value.clone()),
            _ => None,
          }
        } else {
          match property.as_ref() {
            Expression::Identifier { name, .. } => Some(name.clone()),
            _ => None,
          }
        };
        let field_type = match obj_type {
          Type::Enum { members, .. } => {
            if let Some(key) = key {
              members.iter().find(|(name, _)| name == &key).map_or(Type::Any, |(_, t)| t.clone())
            } else {
              Type::Any
            }
          }
          Type::Object { fields } => {
            if let Some(key) = key {
              fields.iter().find(|(name, _)| name == &key).map_or(Type::Any, |(_, t)| t.clone())
            } else {
              Type::Any
            }
          }
          Type::Union(variants) => {
            // Union member access: if all variants are objects and share a field, return union of field types
            if let Some(key) = key {
              let field_types: Vec<Type> = variants
                .iter()
                .filter_map(|v| match v {
                  Type::Object { fields } => {
                    fields.iter().find(|(name, _)| name == &key).map(|(_, t)| t.clone())
                  }
                  _ => None,
                })
                .collect();
              if field_types.len() == variants.len() {
                if field_types.len() == 1 {
                  field_types.into_iter().next().unwrap()
                } else {
                  Type::Union(field_types)
                }
              } else {
                Type::Any
              }
            } else {
              Type::Any
            }
          }
          _ => Type::Any,
        };
        // ponytail: optional chaining wraps result in T | undefined
        if *optional {
          if matches!(field_type, Type::Undefined) {
            field_type
          } else {
            Type::Union(vec![field_type, Type::Undefined])
          }
        } else {
          field_type
        }
      }
      Expression::AssignmentExpression { right, .. } => self.infer_expression(right, env),
      Expression::ParenthesizedExpression { expression, .. } => {
        self.infer_expression(expression, env)
      }
      Expression::ArrowFunction { params, return_type, body, .. } => {
        let param_types: Vec<Type> = params
          .iter()
          .map(|p| {
            if let Some(ann) = &p.type_ann {
              self.type_ann_to_type(ann, env)
            } else if self.is_strict() {
              self.errors.push(TypeError::ImplicitAny { name: p.name.clone(), span: p.span });
              Type::Any
            } else {
              Type::Any
            }
          })
          .collect();
        let ret = return_type.as_ref().map_or(Type::Any, |ann| self.type_ann_to_type(ann, env));
        // Child scope: params are local to the function body
        env.push_scope();
        for (p, pt) in params.iter().zip(&param_types) {
          if let Err(msg) = env.declare(&p.name, pt.clone()) {
            self.errors.push(TypeError::DuplicateIdentifier(msg, p.span));
          }
        }
        match body {
          crate::ast::ArrowFunctionBody::Expression(expr) => {
            self.infer_expression(expr, env);
          }
          crate::ast::ArrowFunctionBody::Block(stmts) => {
            for stmt in stmts {
              self.check_statement(stmt, env);
            }
          }
        }
        env.pop_scope();
        Type::Function { params: param_types, return_type: Box::new(ret) }
      }
      Expression::Placeholder { .. } => Type::Any,
      Expression::TemplateLiteral { expressions, .. } => {
        for expr in expressions {
          self.infer_expression(expr, env);
        }
        Type::String
      }
      Expression::NewExpression { callee, arguments, .. } => {
        self.infer_expression(callee, env);
        for arg in arguments {
          self.infer_expression(arg, env);
        }
        Type::Any
      }
      Expression::SpreadElement { argument, .. } => {
        self.infer_expression(argument, env);
        Type::Any
      }
      Expression::ThisExpression { .. } => self.current_class_type.clone().unwrap_or(Type::Any),
      Expression::SuperExpression { .. } => Type::Any,
      Expression::ObjectPattern { .. } => Type::Any,
      Expression::ArrayPattern { .. } => Type::Any,
      Expression::AwaitExpression { argument, .. } => {
        let inner = self.infer_expression(argument, env);
        match inner {
          Type::Promise(t) => *t,
          _ => inner,
        }
      }
      Expression::AsExpression { expression, type_ann, .. } => {
        self.infer_expression(expression, env);
        self.type_ann_to_type(type_ann, env)
      }
      Expression::SatisfiesExpression { expression, type_ann, .. } => {
        self.infer_expression(expression, env);
        self.type_ann_to_type(type_ann, env)
      }
      Expression::TaggedTemplateLiteral { tag, expressions, .. } => {
        self.infer_expression(tag, env);
        for expr in expressions {
          self.infer_expression(expr, env);
        }
        Type::String
      }
      Expression::ImportMeta { .. } => {
        // ponytail: import.meta has known properties, return Any for now
        Type::Any
      }
      Expression::ImportCall { source, .. } => {
        self.infer_expression(source, env);
        // dynamic import() returns Promise<any> — return Any for now
        Type::Any
      }
      Expression::JsxElement { tag, attributes, children, .. } => {
        self.infer_expression(tag, env);
        for attr in attributes {
          match &attr.value {
            crate::ast::JsxAttributeValue::Expression(e) => {
              self.infer_expression(e, env);
            }
            crate::ast::JsxAttributeValue::Text(_) => {}
          }
        }
        for child in children {
          match child {
            crate::ast::JsxChild::Expression(e) => {
              self.infer_expression(e, env);
            }
            crate::ast::JsxChild::Element(e) => {
              self.infer_expression(e, env);
            }
            crate::ast::JsxChild::Text(_) => {}
          }
        }
        // JSX returns JSX.Element at runtime — return Any for now
        Type::Any
      }
    }
  }

  /// Detect simple type guard patterns in if-conditions.
  /// Returns (`variable_name`, `narrowed_type`) if detected.
  pub(crate) fn detect_type_guard(
    &mut self,
    expr: &Expression,
    env: &mut TypeEnv,
  ) -> Option<(String, Type)> {
    match expr {
      // Binary expressions: typeof, instanceof, ===, !==
      Expression::BinaryExpression { left, operator, right, .. } => {
        match operator {
          BinaryOp::StrictEq | BinaryOp::Eq => {
            // typeof x === "string" pattern
            if let Some(result) = Self::try_narrow_typeof(left, right) {
              return Some(result);
            }
            if let Some(result) = Self::try_narrow_typeof(right, left) {
              return Some(result);
            }
            // x === null / x === undefined
            if let Some(result) = Self::narrow_strict_eq(left, right, env) {
              return Some(result);
            }
            if let Some(result) = Self::narrow_strict_eq(right, left, env) {
              return Some(result);
            }
            None
          }
          BinaryOp::StrictNotEq | BinaryOp::NotEq => {
            // x !== null → remove null from x's type
            if let Some(result) = Self::narrow_strict_neq(left, right, env) {
              return Some(result);
            }
            if let Some(result) = Self::narrow_strict_neq(right, left, env) {
              return Some(result);
            }
            None
          }
          BinaryOp::Instanceof => {
            if let (
              Expression::Identifier { name, .. },
              Expression::Identifier { name: class_name, .. },
            ) = (left.as_ref(), right.as_ref())
            {
              let narrowed = env.lookup(class_name).unwrap_or(Type::Any);
              return Some((name.clone(), narrowed));
            }
            None
          }
          _ => None,
        }
      }
      // Truthiness: if (x) → remove null/undefined from x's type
      Expression::Identifier { name, .. } => {
        if let Some(original) = env.lookup(name) {
          let narrowed = Self::narrow_to_truthy(&original);
          if narrowed != original {
            return Some((name.clone(), narrowed));
          }
        }
        None
      }
      // Negation: if (!x) → keep only null/undefined
      Expression::UnaryExpression { operator: UnaryOp::Not, operand, .. } => {
        if let Expression::Identifier { name, .. } = operand.as_ref()
          && let Some(original) = env.lookup(name)
        {
          let narrowed = Self::narrow_to_falsy(&original);
          if narrowed != original {
            return Some((name.clone(), narrowed));
          }
        }
        None
      }
      _ => None,
    }
  }

  /// Try typeof x === "string" pattern. Returns (name, `narrowed_type`) if matched.
  fn try_narrow_typeof(unary: &Expression, literal: &Expression) -> Option<(String, Type)> {
    if let (
      Expression::UnaryExpression { operator: UnaryOp::TypeOf, operand, .. },
      Expression::StringLiteral { value, .. },
    ) = (unary, literal)
      && let Expression::Identifier { name, .. } = operand.as_ref()
    {
      Self::narrow_from_typeof(name, value)
    } else {
      None
    }
  }

  /// x === null → narrow x to Null; x === undefined → narrow x to Undefined
  fn narrow_strict_eq(
    name_expr: &Expression,
    value_expr: &Expression,
    env: &mut TypeEnv,
  ) -> Option<(String, Type)> {
    if let Expression::Identifier { name, .. } = name_expr {
      let target_type = match value_expr {
        Expression::NullLiteral { .. } => Type::Null,
        Expression::UndefinedLiteral { .. } => Type::Undefined,
        Expression::StringLiteral { value, .. } => {
          Type::Literal(LiteralValue::String(value.clone()))
        }
        Expression::NumberLiteral { value, .. } => Type::Literal(LiteralValue::Number(*value)),
        Expression::BooleanLiteral { value, .. } => Type::Literal(LiteralValue::Boolean(*value)),
        _ => return None,
      };
      // Only narrow if the original type actually contains this variant
      if let Some(original) = env.lookup(name)
        && (original.is_assignable_to(&target_type) || target_type.is_assignable_to(&original))
      {
        return Some((name.clone(), target_type));
      }
    }
    None
  }

  /// x !== null → remove null from x's type; x !== undefined → remove undefined
  fn narrow_strict_neq(
    name_expr: &Expression,
    value_expr: &Expression,
    env: &mut TypeEnv,
  ) -> Option<(String, Type)> {
    if let Expression::Identifier { name, .. } = name_expr {
      let remove_type = match value_expr {
        Expression::NullLiteral { .. } => Some(Type::Null),
        Expression::UndefinedLiteral { .. } => Some(Type::Undefined),
        _ => None,
      };
      if let Some(rm) = remove_type
        && let Some(original) = env.lookup(name)
      {
        let narrowed = Self::remove_from_union(&original, &rm);
        if narrowed != original {
          return Some((name.clone(), narrowed));
        }
      }
    }
    None
  }

  /// Truthiness: remove null/undefined from a type.
  fn narrow_to_truthy(ty: &Type) -> Type {
    match ty {
      Type::Union(types) => {
        let filtered: Vec<Type> =
          types.iter().filter(|t| !matches!(t, Type::Null | Type::Undefined)).cloned().collect();
        match filtered.len() {
          0 => ty.clone(), // can't narrow further
          1 => filtered.into_iter().next().unwrap(),
          _ => Type::Union(filtered),
        }
      }
      Type::Null | Type::Undefined => Type::Never,
      other => other.clone(),
    }
  }

  /// Falsiness: keep only null/undefined/false/0/""
  fn narrow_to_falsy(ty: &Type) -> Type {
    match ty {
      Type::Union(types) => {
        let falsy: Vec<Type> = types.iter().filter(|t| Self::is_falsy_type(t)).cloned().collect();
        match falsy.len() {
          0 => ty.clone(),
          1 => falsy.into_iter().next().unwrap(),
          _ => Type::Union(falsy),
        }
      }
      other if Self::is_falsy_type(other) => other.clone(),
      _ => Type::Never,
    }
  }

  fn is_falsy_type(ty: &Type) -> bool {
    matches!(ty, Type::Null | Type::Undefined)
      || matches!(ty, Type::Literal(LiteralValue::Boolean(false)))
      || matches!(ty, Type::Literal(LiteralValue::Number(n)) if *n == 0.0)
      || matches!(ty, Type::Literal(LiteralValue::String(s)) if s.is_empty())
  }

  /// Remove a type from a union.
  fn remove_from_union(ty: &Type, remove: &Type) -> Type {
    match ty {
      Type::Union(types) => {
        let filtered: Vec<Type> =
          types.iter().filter(|t| *t != remove && !t.is_assignable_to(remove)).cloned().collect();
        match filtered.len() {
          0 => Type::Never,
          1 => filtered.into_iter().next().unwrap(),
          _ => Type::Union(filtered),
        }
      }
      other if other == remove => Type::Never,
      other => other.clone(),
    }
  }

  fn narrow_from_typeof(name: &str, value: &str) -> Option<(String, Type)> {
    let narrowed = match value {
      "string" => Type::String,
      "number" => Type::Number,
      "boolean" => Type::Boolean,
      "undefined" => Type::Undefined,
      "object" => Type::Any,
      _ => return None,
    };
    Some((name.to_string(), narrowed))
  }
}

/// Substitute `TypeParam` references with inferred concrete types.
pub(crate) fn substitute_type_params(ty: &Type, bindings: &[(String, Type)]) -> Type {
  if bindings.is_empty() {
    return ty.clone();
  }
  match ty {
    Type::TypeParam(name, _) => {
      bindings.iter().find(|(n, _)| n == name).map_or_else(|| ty.clone(), |(_, t)| t.clone())
    }
    Type::Promise(inner) => Type::Promise(Box::new(substitute_type_params(inner, bindings))),
    Type::Array(inner) => Type::Array(Box::new(substitute_type_params(inner, bindings))),
    Type::Set { value } => Type::Set { value: Box::new(substitute_type_params(value, bindings)) },
    Type::Map { key, value } => Type::Map {
      key: Box::new(substitute_type_params(key, bindings)),
      value: Box::new(substitute_type_params(value, bindings)),
    },
    Type::Tuple(elems) => {
      Type::Tuple(elems.iter().map(|e| substitute_type_params(e, bindings)).collect())
    }
    Type::Optional(inner) => Type::Optional(Box::new(substitute_type_params(inner, bindings))),
    Type::Function { params, return_type } => Type::Function {
      params: params.iter().map(|p| substitute_type_params(p, bindings)).collect(),
      return_type: Box::new(substitute_type_params(return_type, bindings)),
    },
    Type::Union(types) => {
      Type::Union(types.iter().map(|t| substitute_type_params(t, bindings)).collect())
    }
    other => other.clone(),
  }
}
