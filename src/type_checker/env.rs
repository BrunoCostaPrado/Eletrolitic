use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::type_checker::ty::Type;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeEnv {
  scopes: Vec<HashMap<String, Type>>,
}

impl TypeEnv {
  #[must_use]
  pub fn new() -> Self {
    let mut env = Self { scopes: vec![HashMap::new()] };
    let _ = env.declare("number", Type::Number);
    let _ = env.declare("string", Type::String);
    let _ = env.declare("boolean", Type::Boolean);
    let _ = env.declare("null", Type::Null);
    let _ = env.declare("undefined", Type::Undefined);
    let _ = env.declare("void", Type::Void);
    let _ = env.declare("any", Type::Any);
    let _ = env.declare("never", Type::Never);
    // ponytail: common globals — add as needed
    let _ = env.declare("console", Type::Any);
    env
  }

  pub fn declare(&mut self, name: &str, type_: Type) -> Result<(), String> {
    if let Some(scope) = self.scopes.last_mut() {
      if scope.contains_key(name) {
        return Err(format!("Duplicate identifier '{name}'"));
      }
      scope.insert(name.to_string(), type_);
    }
    Ok(())
  }

  #[must_use]
  pub fn lookup(&self, name: &str) -> Option<Type> {
    for scope in self.scopes.iter().rev() {
      if let Some(t) = scope.get(name) {
        return Some(t.clone());
      }
    }
    None
  }

  pub fn push_scope(&mut self) {
    self.scopes.push(HashMap::new());
  }

  pub fn pop_scope(&mut self) {
    if self.scopes.len() > 1 {
      self.scopes.pop();
    }
  }
}

impl Default for TypeEnv {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn builtin_types_available() {
    let env = TypeEnv::new();
    assert_eq!(env.lookup("number"), Some(Type::Number));
    assert_eq!(env.lookup("string"), Some(Type::String));
    assert_eq!(env.lookup("any"), Some(Type::Any));
  }

  #[test]
  fn declare_and_lookup() {
    let mut env = TypeEnv::new();
    env.declare("x", Type::Number).unwrap();
    assert_eq!(env.lookup("x"), Some(Type::Number));
  }

  #[test]
  fn duplicate_declaration_error() {
    let mut env = TypeEnv::new();
    env.declare("x", Type::Number).unwrap();
    assert!(env.declare("x", Type::String).is_err());
  }
}
