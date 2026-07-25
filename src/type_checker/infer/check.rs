use crate::ast::{Expression, ForInit, LiteralValue, MethodKind, Program, PropertyKey, Statement};
use crate::type_checker::TypeChecker;
use crate::type_checker::env::TypeEnv;
use crate::type_checker::error::TypeError;
use crate::type_checker::ty::Type;

impl TypeChecker {
  pub fn check(&mut self, program: &Program, env: &mut TypeEnv) {
    for stmt in &program.body {
      self.check_statement(stmt, env);
    }
  }

  pub(crate) fn check_statement(&mut self, stmt: &Statement, env: &mut TypeEnv) {
    match stmt {
      Statement::ExpressionStatement { expression, .. } => {
        self.infer_expression(expression, env);
      }
      Statement::VariableDeclaration { declarations, .. } => {
        for decl in declarations {
          let inferred = decl.init.as_ref().map(|init| self.infer_expression(init, env));

          let declared = decl.type_ann.as_ref().map(|ann| {
            let t = self.type_ann_to_type(ann, env);
            if let Some(ref inferred) = inferred
              && !inferred.is_assignable_to(&t)
            {
              self.errors.push(TypeError::NotAssignable {
                target: format!("{t}"),
                value: format!("{inferred}"),
                span: decl.span,
              });
            }
            t
          });

          let final_type = declared.or(inferred).unwrap_or(Type::Any);

          match decl.id.as_ref() {
            Expression::Identifier { name, .. } => {
              if let Err(msg) = env.declare(name, final_type) {
                self.errors.push(TypeError::DuplicateIdentifier(msg, decl.span));
              }
            }
            Expression::ObjectPattern { .. } | Expression::ArrayPattern { .. } => {
              self.declare_pattern_names(decl.id.as_ref(), final_type, env, decl.span);
            }
            _ => {}
          }
        }
      }
      Statement::BlockStatement { body, .. } => {
        env.push_scope();
        for stmt in body {
          self.check_statement(stmt, env);
        }
        env.pop_scope();
      }
      Statement::IfStatement { test, consequent, alternate, .. } => {
        // Type guard narrowing: detect typeof x === "string" and x instanceof Foo
        let guard_narrowing = self.detect_type_guard(test, env);
        if let Some((name, narrowed_type)) = &guard_narrowing {
          self.narrowed.insert(name.clone(), narrowed_type.clone());
        }
        env.push_scope();
        self.check_statement(consequent, env);
        env.pop_scope();
        if let Some((name, _)) = &guard_narrowing {
          self.narrowed.remove(name);
        }
        if let Some(alt) = alternate {
          env.push_scope();
          self.check_statement(alt, env);
          env.pop_scope();
        }
      }
      Statement::WhileStatement { test, body, .. } => {
        self.infer_expression(test, env);
        self.check_statement(body, env);
      }
      Statement::ForStatement { init, test, update, body, .. } => {
        env.push_scope();
        if let Some(ForInit::VariableDeclaration { declarations, .. }) = init {
          for decl in declarations {
            let inferred = decl.init.as_ref().map(|init| self.infer_expression(init, env));
            let final_type = inferred.unwrap_or(Type::Any);
            match decl.id.as_ref() {
              Expression::Identifier { name, .. } => {
                if let Err(msg) = env.declare(name, final_type) {
                  self.errors.push(TypeError::DuplicateIdentifier(msg, decl.span));
                }
              }
              Expression::ObjectPattern { .. } | Expression::ArrayPattern { .. } => {
                self.declare_pattern_names(decl.id.as_ref(), final_type, env, decl.span);
              }
              _ => {}
            }
          }
        } else if let Some(ForInit::Expression(expr)) = init {
          self.infer_expression(expr, env);
        }
        if let Some(t) = test {
          self.infer_expression(t, env);
        }
        if let Some(u) = update {
          self.infer_expression(u, env);
        }
        self.check_statement(body, env);
        env.pop_scope();
      }
      Statement::ReturnStatement { value, span, .. } => {
        if let Some(v) = value {
          let ret_type = self.infer_expression(v, env);
          if let Some(expected) = &self.current_return_type
            && !ret_type.is_assignable_to(expected)
          {
            self.errors.push(TypeError::NotAssignable {
              target: format!("{expected}"),
              value: format!("{ret_type}"),
              span: *span,
            });
          }
        } else if let Some(expected) = &self.current_return_type {
          // ponytail: void return vs non-void expected
          if !matches!(expected, Type::Void) && !matches!(expected, Type::Any) {
            self.errors.push(TypeError::NotAssignable {
              target: format!("{expected}"),
              value: "void".to_string(),
              span: *span,
            });
          }
        }
      }
      Statement::FunctionDeclaration {
        name,
        params,
        return_type,
        body,
        is_async,
        type_params,
        ..
      } => {
        // Resolve param types in outer scope, declare function name in outer scope
        for (tp, constraint) in type_params {
          let c = constraint.as_ref().map(|ann| self.type_ann_to_type(ann, env));
          let _ = env.declare(tp, Type::TypeParam(tp.clone(), c.map(Box::new)));
        }
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
        // ponytail: async fn with Promise<T> annotation → unwrap T for body checking
        let body_return = if *is_async {
          match &ret {
            Type::Promise(inner) => (**inner).clone(),
            _ => ret.clone(),
          }
        } else {
          ret.clone()
        };
        let fn_type = Type::Function { params: param_types.clone(), return_type: Box::new(ret) };
        if let Err(msg) = env.declare(name, fn_type) {
          self.errors.push(TypeError::DuplicateIdentifier(msg, params[0].span));
        }
        // Child scope: params are local to the function body
        env.push_scope();
        for (p, pt) in params.iter().zip(&param_types) {
          if let Err(msg) = env.declare(&p.name, pt.clone()) {
            self.errors.push(TypeError::DuplicateIdentifier(msg, p.span));
          }
        }
        let prev_ret = self.current_return_type.replace(body_return);
        self.check_statement(body, env);
        self.current_return_type = prev_ret;
        env.pop_scope();
      }
      Statement::ImportDeclaration { specifiers, source, .. } => {
        // Resolve import source to module registry key
        let resolved_key = if source.starts_with('.') && !self.current_module.as_os_str().is_empty()
        {
          self
            .current_module
            .parent()
            .unwrap_or(&self.current_module)
            .join(source)
            .with_extension("ts")
            .to_string_lossy()
            .to_string()
        } else if let Some(resolved) =
          crate::config::resolve_path_alias(source, source, &self.options)
        {
          self.root_dir.join(resolved).to_string_lossy().to_string()
        } else {
          source.clone()
        };
        let exports = self.module_registry.get(&resolved_key);
        for s in specifiers {
          let ty = if s.is_default {
            exports.and_then(|e| e.get("default")).cloned().unwrap_or(Type::Any)
          } else {
            let name = s.imported.as_ref().unwrap_or(&s.local);
            exports.and_then(|e| e.get(name)).cloned().unwrap_or(Type::Any)
          };
          let _ = env.declare(&s.local, ty);
        }
      }
      Statement::TypeAliasDeclaration { name, type_annotation, .. } => {
        let resolved = self.type_ann_to_type(type_annotation, env);
        let _ = env.declare(name, resolved);
      }
      Statement::InterfaceDeclaration { .. } => {
        // ponytail: interfaces are type-only, skipped at runtime
      }
      Statement::ExportDeclaration { declaration, .. } => {
        self.check_statement(declaration, env);
      }
      Statement::ForInOfStatement { left, right, body, is_of, .. } => {
        let right_type = self.infer_expression(right, env);
        let elem_type = if *is_of {
          match &right_type {
            Type::Array(elem) => (**elem).clone(),
            Type::Set { value } => (**value).clone(),
            Type::Map { key, value } => Type::Tuple(vec![(**key).clone(), (**value).clone()]),
            Type::String => Type::String,
            _ => Type::Any,
          }
        } else {
          // ponytail: for-in keys are always string in TS
          Type::String
        };
        env.push_scope();
        if let Expression::Identifier { name, .. } = left {
          let _ = env.declare(&name, elem_type);
        }
        self.check_statement(body, env);
        env.pop_scope();
      }
      Statement::SwitchStatement { discriminant, cases, .. } => {
        let disc_type = self.infer_expression(discriminant, env);
        let disc_prim = disc_type.primitive();
        for case in cases {
          if let Some(test) = &case.test {
            let case_type = self.infer_expression(test, env);
            let case_prim = case_type.primitive();
            // ponytail: check at primitive level — Literal("1") vs Literal("2") is fine
            if !disc_prim.is_assignable_to(&case_prim) && !case_prim.is_assignable_to(&disc_prim) {
              self.errors.push(TypeError::NotAssignable {
                target: format!("{disc_type}"),
                value: format!("{case_type}"),
                span: test.span(),
              });
            }
          }
          for stmt in &case.body {
            self.check_statement(stmt, env);
          }
        }
      }
      Statement::ThrowStatement { argument, .. } => {
        self.infer_expression(argument, env);
      }
      Statement::TryStatement { body, handler, finalizer, .. } => {
        self.check_statement(body, env);
        if let Some(catch) = handler {
          env.push_scope();
          let catch_type =
            catch.type_ann.as_ref().map_or(Type::Unknown, |ann| self.type_ann_to_type(ann, env));
          if let Some(ref param) = catch.param {
            let _ = env.declare(param, catch_type);
          }
          for stmt in &catch.body {
            self.check_statement(stmt, env);
          }
          env.pop_scope();
        }
        if let Some(finalizer_body) = finalizer {
          for stmt in finalizer_body {
            self.check_statement(stmt, env);
          }
        }
      }
      Statement::BreakStatement { .. } | Statement::ContinueStatement { .. } => {}
      Statement::LabeledStatement { body, .. } => {
        self.check_statement(body, env);
      }
      Statement::DoWhileStatement { test, body, .. } => {
        self.check_statement(body, env);
        self.infer_expression(test, env);
      }
      Statement::ClassDeclaration { name, superclass, body, .. } => {
        // Infer superclass methods first
        let super_fields = if let Some(superclass) = superclass {
          let super_type = self.infer_expression(superclass, env);
          match super_type {
            Type::Object { fields } => fields,
            _ => Vec::new(),
          }
        } else {
          Vec::new()
        };
        // Build instance type from fields and methods
        let mut instance_fields = super_fields;
        // Add fields
        for field in &body.fields {
          let field_type = field
            .type_ann
            .as_ref()
            .map(|ann| self.type_ann_to_type(ann, env))
            .or_else(|| field.init.as_ref().map(|init| self.infer_expression(init, env)))
            .unwrap_or(Type::Any);
          let field_name = match &field.key {
            PropertyKey::Identifier(n) => n.clone(),
            PropertyKey::String(n) => n.clone(),
            PropertyKey::Expression(_) => continue,
          };
          if let Some(existing) = instance_fields.iter_mut().find(|(k, _)| *k == field_name) {
            existing.1 = field_type;
          } else {
            instance_fields.push((field_name, field_type));
          }
        }
        // Add methods
        for method in &body.methods {
          if let MethodKind::Constructor = method.kind {
            continue;
          }
          let param_types: Vec<Type> = method
            .params
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
          let method_ret =
            method.return_type.as_ref().map_or(Type::Any, |ann| self.type_ann_to_type(ann, env));
          let method_type =
            Type::Function { params: param_types, return_type: Box::new(method_ret) };
          let method_name = match &method.key {
            PropertyKey::Identifier(n) => n.clone(),
            PropertyKey::String(n) => n.clone(),
            PropertyKey::Expression(_) => continue,
          };
          if let Some(existing) = instance_fields.iter_mut().find(|(k, _)| *k == method_name) {
            existing.1 = method_type;
          } else {
            instance_fields.push((method_name, method_type));
          }
        }
        let instance_type = Type::Object { fields: instance_fields };
        let _ = env.declare(name, instance_type.clone());
        let prev_class = self.current_class_type.replace(instance_type);
        for method in &body.methods {
          env.push_scope();
          for param in &method.params {
            let param_type =
              param.type_ann.as_ref().map_or(Type::Any, |ann| self.type_ann_to_type(ann, env));
            let _ = env.declare(&param.name, param_type);
          }
          let method_ret =
            method.return_type.as_ref().map(|ann| self.type_ann_to_type(ann, env)).unwrap_or(
              if matches!(method.kind, MethodKind::Constructor) { Type::Void } else { Type::Any },
            );
          let prev_ret = self.current_return_type.replace(method_ret);
          self.check_statement(&method.body, env);
          self.current_return_type = prev_ret;
          env.pop_scope();
        }
        self.current_class_type = prev_class;
      }
      Statement::EnumDeclaration { name, members, .. } => {
        // Build enum type with member types
        let mut enum_members = Vec::new();
        let mut member_num = 0.0;
        for m in members {
          let member_type = if let Some(val) = &m.value {
            // String enum member with explicit value
            if val.starts_with('"') || val.starts_with('\'') {
              Type::Literal(LiteralValue::String(
                val.trim_matches(|c| c == '"' || c == '\'').to_string(),
              ))
            } else if let Ok(n) = val.parse::<f64>() {
              Type::Literal(LiteralValue::Number(n))
            } else {
              Type::Number
            }
          } else {
            // Auto-incremented numeric enum
            let t = Type::Literal(LiteralValue::Number(member_num));
            member_num += 1.0;
            t
          };
          enum_members.push((m.name.clone(), member_type.clone()));
          // Also declare member as a variable for backward compat
          let _ = env.declare(&m.name, member_type);
        }
        let _ = env.declare(name, Type::Enum { name: name.clone(), members: enum_members });
      }
      Statement::DeclareModule { body, .. } | Statement::DeclareNamespace { body, .. } => {
        for stmt in body {
          self.check_statement(stmt, env);
        }
      }
      Statement::DecoratedStatement { statement, .. } => {
        self.check_statement(statement, env);
      }
    }
  }
}
