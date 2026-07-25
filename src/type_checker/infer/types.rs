use crate::ast::{Expression, LiteralValue, PropertyKey, TypeAnn};
use crate::token::Span;
use crate::type_checker::TypeChecker;
use crate::type_checker::env::TypeEnv;
use crate::type_checker::error::TypeError;
use crate::type_checker::ty::Type;

impl TypeChecker {
  pub(crate) fn type_ann_to_type(&mut self, ann: &TypeAnn, env: &mut TypeEnv) -> Type {
    match ann {
      TypeAnn::TypeRef { name, type_args } => {
        // Utility types: Partial<T>, Pick<T, K>
        match name.as_str() {
          "Promise" if type_args.len() == 1 => {
            Type::Promise(Box::new(self.type_ann_to_type(&type_args[0], env)))
          }
          "Map" if type_args.len() == 2 => Type::Map {
            key: Box::new(self.type_ann_to_type(&type_args[0], env)),
            value: Box::new(self.type_ann_to_type(&type_args[1], env)),
          },
          "Set" if type_args.len() == 1 => {
            Type::Set { value: Box::new(self.type_ann_to_type(&type_args[0], env)) }
          }
          "Partial" if type_args.len() == 1 => {
            let inner = self.type_ann_to_type(&type_args[0], env);
            match inner {
              Type::Object { fields } => {
                let optional_fields: Vec<(String, Type)> = fields
                  .into_iter()
                  .map(|(name, ty)| (name, Type::Optional(Box::new(ty))))
                  .collect();
                Type::Object { fields: optional_fields }
              }
              Type::TypeParam(name, constraint) => Type::TypeParam(name, constraint),
              _ => Type::Any,
            }
          }
          "Pick" if type_args.len() == 2 => {
            let inner = self.type_ann_to_type(&type_args[0], env);
            let keys = self.type_ann_to_type(&type_args[1], env);
            match (inner, keys) {
              (Type::Object { fields }, Type::Union(key_types)) => {
                let picked: Vec<(String, Type)> = key_types
                  .into_iter()
                  .filter_map(|kt| match kt {
                    Type::Literal(LiteralValue::String(s)) => {
                      fields.iter().find(|(k, _)| *k == s).map(|(k, t)| (k.clone(), t.clone()))
                    }
                    _ => None,
                  })
                  .collect();
                Type::Object { fields: picked }
              }
              (Type::Object { fields }, Type::Literal(LiteralValue::String(s))) => {
                let picked: Vec<(String, Type)> = fields
                  .iter()
                  .filter(|(k, _)| *k == s)
                  .map(|(k, t)| (k.clone(), t.clone()))
                  .collect();
                Type::Object { fields: picked }
              }
              _ => Type::Any,
            }
          }
          "Omit" if type_args.len() == 2 => {
            let inner = self.type_ann_to_type(&type_args[0], env);
            let keys = self.type_ann_to_type(&type_args[1], env);
            match (inner, keys) {
              (Type::Object { fields }, Type::Union(key_types)) => {
                let key_names: Vec<String> = key_types
                  .into_iter()
                  .filter_map(|kt| match kt {
                    Type::Literal(LiteralValue::String(s)) => Some(s),
                    _ => None,
                  })
                  .collect();
                let omitted: Vec<(String, Type)> =
                  fields.into_iter().filter(|(k, _)| !key_names.contains(k)).collect();
                Type::Object { fields: omitted }
              }
              (Type::Object { fields }, Type::Literal(LiteralValue::String(s))) => {
                let omitted: Vec<(String, Type)> =
                  fields.into_iter().filter(|(k, _)| *k != s).collect();
                Type::Object { fields: omitted }
              }
              _ => Type::Any,
            }
          }
          _ => env.lookup(name).unwrap_or(Type::Any),
        }
      }
      TypeAnn::Typeof { argument } => self.infer_expression(argument, env),
      TypeAnn::KeyOf { type_ann } => {
        let inner = self.type_ann_to_type(type_ann, env);
        match inner {
          Type::Object { fields } => {
            // keyof produces union of string literal keys
            let key_types: Vec<Type> =
              fields.iter().map(|(k, _)| Type::Literal(LiteralValue::String(k.clone()))).collect();
            if key_types.is_empty() {
              Type::String
            } else if key_types.len() == 1 {
              key_types.into_iter().next().unwrap()
            } else {
              Type::Union(key_types)
            }
          }
          Type::Any | Type::TypeParam(..) => Type::String,
          _ => Type::String,
        }
      }
      TypeAnn::Conditional { check, extends, true_type, false_type } => {
        let check_type = self.type_ann_to_type(check, env);
        let extends_type = self.type_ann_to_type(extends, env);
        if check_type.is_assignable_to(&extends_type) {
          self.type_ann_to_type(true_type, env)
        } else {
          self.type_ann_to_type(false_type, env)
        }
      }
      TypeAnn::Infer { name } => Type::TypeParam(name.clone(), None),
      TypeAnn::Mapped { key: _, target, value } => {
        let target_type = self.type_ann_to_type(target, env);
        match target_type {
          Type::Object { fields } => {
            let mapped_fields: Vec<(String, Type)> = fields
              .iter()
              .map(|(field_name, _field_type)| {
                // Replace key param with field name in value type
                let val = self.type_ann_to_type(value, env);
                (field_name.clone(), val)
              })
              .collect();
            Type::Object { fields: mapped_fields }
          }
          _ => Type::Any,
        }
      }
      TypeAnn::IndexedAccess { target, index } => {
        let target_type = self.type_ann_to_type(target, env);
        let index_type = self.type_ann_to_type(index, env);
        match (target_type, index_type) {
          (Type::Object { fields }, Type::Literal(LiteralValue::String(key))) => {
            fields.iter().find(|(k, _)| *k == key).map_or(Type::Any, |(_, t)| t.clone())
          }
          (Type::Object { fields }, _) => {
            // Union of all field types
            let types: Vec<Type> = fields.into_iter().map(|(_, t)| t).collect();
            if types.len() == 1 { types.into_iter().next().unwrap() } else { Type::Union(types) }
          }
          (Type::Array(element), _) => *element,
          _ => Type::Any,
        }
      }
      TypeAnn::Intersection { types } => {
        let resolved: Vec<Type> = types.iter().map(|t| self.type_ann_to_type(t, env)).collect();
        // Merge all object fields; if any non-object, return Any
        let mut merged_fields: Vec<(String, Type)> = Vec::new();
        for t in &resolved {
          match t {
            Type::Object { fields } => {
              for (k, v) in fields {
                if !merged_fields.iter().any(|(mk, _)| mk == k) {
                  merged_fields.push((k.clone(), v.clone()));
                }
              }
            }
            _ => return Type::Any,
          }
        }
        Type::Object { fields: merged_fields }
      }
      _ => Type::from(ann),
    }
  }

  pub(crate) fn declare_pattern_names(
    &mut self,
    pattern: &Expression,
    ty: Type,
    env: &mut TypeEnv,
    span: Span,
  ) {
    match pattern {
      Expression::Identifier { name, .. } => {
        if let Err(msg) = env.declare(name, ty) {
          self.errors.push(TypeError::DuplicateIdentifier(msg, span));
        }
      }
      Expression::ObjectPattern { properties, .. } => {
        // Decompose object fields: { name, age } from { name: string, age: number }
        for prop in properties {
          let key_name = match &prop.key {
            PropertyKey::Identifier(n) | PropertyKey::String(n) => n.clone(),
            PropertyKey::Expression(_) => continue,
          };
          let field_type = match &ty {
            Type::Object { fields } => {
              fields.iter().find(|(k, _)| *k == key_name).map_or(Type::Any, |(_, t)| t.clone())
            }
            _ => Type::Any,
          };
          self.declare_pattern_names(&prop.value, field_type, env, span);
        }
      }
      Expression::ArrayPattern { elements, .. } => {
        // Decompose tuple/array elements: [a, b] from [string, number]
        for (i, e) in elements.iter().enumerate() {
          if let Some(elem) = e {
            let elem_type = match &ty {
              Type::Tuple(elems) => elems.get(i).cloned().unwrap_or(Type::Any),
              Type::Array(inner) => (**inner).clone(),
              _ => Type::Any,
            };
            self.declare_pattern_names(elem, elem_type, env, span);
          }
        }
      }
      _ => {}
    }
  }
}
