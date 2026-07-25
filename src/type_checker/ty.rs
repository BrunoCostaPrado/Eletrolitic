use crate::ast::LiteralValue;
use crate::ast::TypeAnn;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Number,
  String,
  Boolean,
  Null,
  Undefined,
  Void,
  Any,
  Unknown,
  Never,
  Literal(LiteralValue),
  Union(Vec<Type>),
  Array(Box<Type>),
  Function { params: Vec<Type>, return_type: Box<Type> },
  Object { fields: Vec<(String, Type)> },
  TypeParam(String, Option<Box<Type>>),
  Promise(Box<Type>),
  Map { key: Box<Type>, value: Box<Type> },
  Set { value: Box<Type> },
  Tuple(Vec<Type>),
  Optional(Box<Type>),
  Enum { name: String, members: Vec<(String, Type)> },
}

impl Type {
  /// Returns the primitive base type, stripping literal wrappers.
  /// e.g. Literal(Number(1)) → Number, Literal(String("a")) → String
  #[must_use]
  pub fn primitive(&self) -> Type {
    match self {
      Type::Literal(LiteralValue::Number(_)) => Type::Number,
      Type::Literal(LiteralValue::String(_)) => Type::String,
      Type::Literal(LiteralValue::Boolean(_)) => Type::Boolean,
      other => other.clone(),
    }
  }

  #[must_use]
  pub fn is_assignable_to(&self, target: &Type) -> bool {
    if matches!(self, Type::Any) || matches!(target, Type::Any) {
      return true;
    }
    if matches!(self, Type::Never) {
      return true;
    }
    if matches!(target, Type::Unknown) {
      return true;
    }
    match (self, target) {
      (a, b) if a == b => true,
      (Type::Literal(LiteralValue::Number(_)), Type::Number) => true,
      (Type::Literal(LiteralValue::String(_)), Type::String) => true,
      (Type::Literal(LiteralValue::Boolean(_)), Type::Boolean) => true,
      (Type::Union(types), _) => types.iter().all(|t| t.is_assignable_to(target)),
      (_, Type::Union(types)) => types.iter().any(|t| self.is_assignable_to(t)),
      (Type::Array(a), Type::Array(b)) => a.is_assignable_to(b),
      (Type::Object { fields: a }, Type::Object { fields: b }) => {
        // structural: a must have all required fields of b with assignable types;
        // optional fields in b may be missing from a
        b.iter().all(|(bname, btype)| {
          let (bcore, is_optional) = match btype {
            Type::Optional(inner) => (inner.as_ref(), true),
            other => (other, false),
          };
          if let Some((_, atype)) = a.iter().find(|(aname, _)| aname == bname) {
            let acore = match atype {
              Type::Optional(inner) => inner.as_ref(),
              other => other,
            };
            if is_optional {
              acore.is_assignable_to(bcore) || matches!(acore, Type::Undefined)
            } else {
              acore.is_assignable_to(bcore)
            }
          } else {
            is_optional
          }
        })
      }
      (Type::Null, Type::Undefined) => true,
      (Type::Undefined, Type::Null) => true,
      (Type::Null, Type::Void) => true,
      (Type::Undefined, Type::Void) => true,
      (Type::Promise(a), Type::Promise(b)) => a.is_assignable_to(b),
      (Type::Map { key: ka, value: va }, Type::Map { key: kb, value: vb }) => {
        ka.is_assignable_to(kb) && va.is_assignable_to(vb)
      }
      (Type::Set { value: va }, Type::Set { value: vb }) => va.is_assignable_to(vb),
      (Type::Tuple(a), Type::Tuple(b)) => {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a.is_assignable_to(b))
      }
      (Type::Enum { members: ma, .. }, Type::Enum { members: mb, .. }) => {
        mb.iter().all(|(bname, btype)| {
          ma.iter()
            .find(|(aname, _)| aname == bname)
            .is_some_and(|(_, atype)| atype.is_assignable_to(btype))
        })
      }
      _ => false,
    }
  }
}

impl std::fmt::Display for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Type::Number => write!(f, "number"),
      Type::String => write!(f, "string"),
      Type::Boolean => write!(f, "boolean"),
      Type::Null => write!(f, "null"),
      Type::Undefined => write!(f, "undefined"),
      Type::Void => write!(f, "void"),
      Type::Any => write!(f, "any"),
      Type::Unknown => write!(f, "unknown"),
      Type::Never => write!(f, "never"),
      Type::Literal(lit) => match lit {
        LiteralValue::Number(n) => write!(f, "{n}"),
        LiteralValue::String(s) => write!(f, "\"{s}\""),
        LiteralValue::Boolean(b) => write!(f, "{b}"),
      },
      Type::Union(types) => {
        for (i, t) in types.iter().enumerate() {
          if i > 0 {
            write!(f, " | ")?;
          }
          write!(f, "{t}")?;
        }
        Ok(())
      }
      Type::Array(elem) => write!(f, "{elem}[]"),
      Type::Promise(inner) => write!(f, "Promise<{inner}>"),
      Type::Function { params, return_type } => {
        write!(f, "(")?;
        for (i, p) in params.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{p}")?;
        }
        write!(f, ") => {return_type}")
      }
      Type::Object { fields } => {
        write!(f, "{{ ")?;
        for (i, (name, ty)) in fields.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          match ty {
            Type::Optional(inner) => write!(f, "{name}?: {inner}")?,
            _ => write!(f, "{name}: {ty}")?,
          }
        }
        write!(f, " }}")
      }
      Type::TypeParam(name, constraint) => {
        if let Some(c) = constraint {
          write!(f, "{name} extends {c}")
        } else {
          write!(f, "{name}")
        }
      }
      Type::Map { key, value } => write!(f, "Map<{key}, {value}>"),
      Type::Set { value } => write!(f, "Set<{value}>"),
      Type::Tuple(elems) => {
        write!(f, "[")?;
        for (i, e) in elems.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{e}")?;
        }
        write!(f, "]")
      }
      Type::Optional(inner) => write!(f, "{inner}?"),
      Type::Enum { name, .. } => write!(f, "{name}"),
    }
  }
}

impl From<&TypeAnn> for Type {
  fn from(ann: &TypeAnn) -> Self {
    match ann {
      TypeAnn::Number => Type::Number,
      TypeAnn::String => Type::String,
      TypeAnn::Boolean => Type::Boolean,
      TypeAnn::Null => Type::Null,
      TypeAnn::Undefined => Type::Undefined,
      TypeAnn::Void => Type::Void,
      TypeAnn::Any => Type::Any,
      TypeAnn::Unknown => Type::Unknown,
      TypeAnn::Never => Type::Never,
      TypeAnn::Literal { value } => Type::Literal(value.clone()),
      TypeAnn::Union { types } => Type::Union(types.iter().map(Type::from).collect()),
      TypeAnn::Intersection { .. } => Type::Any,
      TypeAnn::TemplateLiteral { .. } => Type::String,
      TypeAnn::Array { element } => Type::Array(Box::new(Type::from(element.as_ref()))),
      TypeAnn::TypeRef { name, type_args } => match name.as_str() {
        "Map" if type_args.len() == 2 => Type::Map {
          key: Box::new(Type::from(&type_args[0])),
          value: Box::new(Type::from(&type_args[1])),
        },
        "Set" if type_args.len() == 1 => Type::Set { value: Box::new(Type::from(&type_args[0])) },
        _ => Type::TypeParam(name.clone(), None),
      },
      TypeAnn::Typeof { .. } => Type::Any,
      TypeAnn::KeyOf { .. } => Type::String,
      TypeAnn::Conditional { .. } => Type::Any,
      TypeAnn::Infer { name } => Type::TypeParam(name.clone(), None),
      TypeAnn::Mapped { .. } => Type::Any, // handled in type_ann_to_type
      TypeAnn::IndexedAccess { .. } => Type::Any, // handled in type_ann_to_type
      TypeAnn::Function { params, return_type } => Type::Function {
        params: params.iter().map(Type::from).collect(),
        return_type: Box::new(Type::from(return_type.as_ref())),
      },
      TypeAnn::Object { properties } => Type::Object {
        fields: properties.iter().map(|p| (p.name.clone(), Type::from(&p.type_ann))).collect(),
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn number_assignable_to_number() {
    assert!(Type::Number.is_assignable_to(&Type::Number));
  }

  #[test]
  fn number_not_assignable_to_string() {
    assert!(!Type::Number.is_assignable_to(&Type::String));
  }

  #[test]
  fn literal_assignable_to_base() {
    assert!(Type::Literal(LiteralValue::Number(42.0)).is_assignable_to(&Type::Number));
    assert!(Type::Literal(LiteralValue::String("hi".into())).is_assignable_to(&Type::String));
    assert!(Type::Literal(LiteralValue::Boolean(true)).is_assignable_to(&Type::Boolean));
  }

  #[test]
  fn any_assignable_to_anything() {
    assert!(Type::Any.is_assignable_to(&Type::Number));
    assert!(Type::Any.is_assignable_to(&Type::String));
    assert!(Type::Number.is_assignable_to(&Type::Any));
  }

  #[test]
  fn never_assignable_to_anything() {
    assert!(Type::Never.is_assignable_to(&Type::Number));
    assert!(Type::Never.is_assignable_to(&Type::Any));
  }

  #[test]
  fn assignable_to_union() {
    let union = Type::Union(vec![Type::Number, Type::String]);
    assert!(Type::Number.is_assignable_to(&union));
    assert!(Type::String.is_assignable_to(&union));
    assert!(!Type::Boolean.is_assignable_to(&union));
  }

  #[test]
  fn array_assignable() {
    assert!(
      Type::Array(Box::new(Type::Number)).is_assignable_to(&Type::Array(Box::new(Type::Number)))
    );
    assert!(
      !Type::Array(Box::new(Type::Number)).is_assignable_to(&Type::Array(Box::new(Type::String)))
    );
  }

  #[test]
  fn null_undefined_void_assignability() {
    assert!(Type::Null.is_assignable_to(&Type::Undefined));
    assert!(Type::Undefined.is_assignable_to(&Type::Null));
    assert!(Type::Null.is_assignable_to(&Type::Void));
    assert!(Type::Undefined.is_assignable_to(&Type::Void));
    assert!(!Type::Void.is_assignable_to(&Type::Null));
  }

  #[test]
  fn type_ann_conversion() {
    assert_eq!(Type::from(&TypeAnn::Number), Type::Number);
    assert_eq!(Type::from(&TypeAnn::String), Type::String);
    assert_eq!(Type::from(&TypeAnn::Boolean), Type::Boolean);
    assert_eq!(Type::from(&TypeAnn::Null), Type::Null);
    assert_eq!(Type::from(&TypeAnn::Undefined), Type::Undefined);
    assert_eq!(Type::from(&TypeAnn::Void), Type::Void);
    assert_eq!(Type::from(&TypeAnn::Any), Type::Any);
    assert_eq!(Type::from(&TypeAnn::Never), Type::Never);
  }

  #[test]
  fn type_ann_literal_conversion() {
    assert_eq!(
      Type::from(&TypeAnn::Literal { value: LiteralValue::Number(42.0) }),
      Type::Literal(LiteralValue::Number(42.0))
    );
  }

  #[test]
  fn type_ann_union_conversion() {
    let ann = TypeAnn::Union { types: vec![TypeAnn::Number, TypeAnn::String] };
    let expected = Type::Union(vec![Type::Number, Type::String]);
    assert_eq!(Type::from(&ann), expected);
  }

  #[test]
  fn type_ann_array_conversion() {
    let ann = TypeAnn::Array { element: Box::new(TypeAnn::Number) };
    assert_eq!(Type::from(&ann), Type::Array(Box::new(Type::Number)));
  }

  #[test]
  fn type_ann_typeref_conversion() {
    let ann = TypeAnn::TypeRef { name: "Foo".into(), type_args: Vec::new() };
    assert_eq!(Type::from(&ann), Type::TypeParam("Foo".into(), None));
  }

  #[test]
  fn display_types() {
    assert_eq!(format!("{}", Type::Number), "number");
    assert_eq!(format!("{}", Type::Literal(LiteralValue::Number(42.0))), "42");
    assert_eq!(format!("{}", Type::Literal(LiteralValue::String("hi".into()))), "\"hi\"");
    assert_eq!(format!("{}", Type::Union(vec![Type::Number, Type::String])), "number | string");
    assert_eq!(format!("{}", Type::Array(Box::new(Type::Number))), "number[]");
  }

  #[test]
  fn source_union_assignable_to_base() {
    let union = Type::Union(vec![
      Type::Literal(LiteralValue::Number(1.0)),
      Type::Literal(LiteralValue::Number(2.0)),
      Type::Literal(LiteralValue::Number(3.0)),
    ]);
    assert!(union.is_assignable_to(&Type::Number));
  }

  #[test]
  fn source_union_not_assignable_when_mixed() {
    let union = Type::Union(vec![Type::Number, Type::String]);
    assert!(!union.is_assignable_to(&Type::Number));
  }

  #[test]
  fn source_union_assignable_to_target_union() {
    let src = Type::Union(vec![Type::Number, Type::String]);
    let tgt = Type::Union(vec![Type::Number, Type::String, Type::Boolean]);
    assert!(src.is_assignable_to(&tgt));
  }

  #[test]
  fn object_structural_assignable() {
    let a =
      Type::Object { fields: vec![("name".into(), Type::String), ("age".into(), Type::Number)] };
    let b = Type::Object { fields: vec![("name".into(), Type::String)] };
    assert!(a.is_assignable_to(&b));
  }

  #[test]
  fn object_missing_field_not_assignable() {
    let a = Type::Object { fields: vec![("name".into(), Type::String)] };
    let b =
      Type::Object { fields: vec![("name".into(), Type::String), ("age".into(), Type::Number)] };
    assert!(!a.is_assignable_to(&b));
  }

  #[test]
  fn object_field_type_mismatch_not_assignable() {
    let a = Type::Object { fields: vec![("name".into(), Type::Number)] };
    let b = Type::Object { fields: vec![("name".into(), Type::String)] };
    assert!(!a.is_assignable_to(&b));
  }

  #[test]
  fn display_object_type() {
    let obj =
      Type::Object { fields: vec![("name".into(), Type::String), ("age".into(), Type::Number)] };
    assert_eq!(format!("{obj}"), "{ name: string, age: number }");
  }

  #[test]
  fn unknown_display() {
    assert_eq!(format!("{}", Type::Unknown), "unknown");
  }

  #[test]
  fn everything_assignable_to_unknown() {
    assert!(Type::Number.is_assignable_to(&Type::Unknown));
    assert!(Type::String.is_assignable_to(&Type::Unknown));
    assert!(Type::Any.is_assignable_to(&Type::Unknown));
    assert!(Type::Never.is_assignable_to(&Type::Unknown));
    assert!(Type::Unknown.is_assignable_to(&Type::Unknown));
  }

  #[test]
  fn unknown_not_assignable_to_concrete() {
    assert!(!Type::Unknown.is_assignable_to(&Type::Number));
    assert!(!Type::Unknown.is_assignable_to(&Type::String));
    assert!(!Type::Unknown.is_assignable_to(&Type::Boolean));
  }

  #[test]
  fn type_ann_unknown_conversion() {
    assert_eq!(Type::from(&TypeAnn::Unknown), Type::Unknown);
  }

  #[test]
  fn primitive_strips_literals() {
    assert_eq!(Type::Literal(LiteralValue::Number(1.0)).primitive(), Type::Number);
    assert_eq!(Type::Literal(LiteralValue::String("a".into())).primitive(), Type::String);
    assert_eq!(Type::Literal(LiteralValue::Boolean(true)).primitive(), Type::Boolean);
    assert_eq!(Type::Number.primitive(), Type::Number);
    assert_eq!(Type::String.primitive(), Type::String);
  }

  #[test]
  fn map_display() {
    let m = Type::Map { key: Box::new(Type::String), value: Box::new(Type::Number) };
    assert_eq!(format!("{m}"), "Map<string, number>");
  }

  #[test]
  fn set_display() {
    let s = Type::Set { value: Box::new(Type::String) };
    assert_eq!(format!("{s}"), "Set<string>");
  }

  #[test]
  fn tuple_display() {
    let t = Type::Tuple(vec![Type::String, Type::Number]);
    assert_eq!(format!("{t}"), "[string, number]");
  }

  #[test]
  fn map_assignable() {
    let a = Type::Map { key: Box::new(Type::String), value: Box::new(Type::Number) };
    let b = Type::Map { key: Box::new(Type::String), value: Box::new(Type::Number) };
    assert!(a.is_assignable_to(&b));
    let c = Type::Map { key: Box::new(Type::String), value: Box::new(Type::String) };
    assert!(!a.is_assignable_to(&c));
  }

  #[test]
  fn set_assignable() {
    let a = Type::Set { value: Box::new(Type::Number) };
    let b = Type::Set { value: Box::new(Type::Number) };
    assert!(a.is_assignable_to(&b));
    let c = Type::Set { value: Box::new(Type::String) };
    assert!(!a.is_assignable_to(&c));
  }

  #[test]
  fn tuple_assignable() {
    let a = Type::Tuple(vec![Type::String, Type::Number]);
    let b = Type::Tuple(vec![Type::String, Type::Number]);
    assert!(a.is_assignable_to(&b));
    let c = Type::Tuple(vec![Type::String, Type::String]);
    assert!(!a.is_assignable_to(&c));
    let d = Type::Tuple(vec![Type::String]);
    assert!(!a.is_assignable_to(&d));
  }

  #[test]
  fn type_ann_map_conversion() {
    let ann =
      TypeAnn::TypeRef { name: "Map".into(), type_args: vec![TypeAnn::String, TypeAnn::Number] };
    let t = Type::from(&ann);
    assert_eq!(t, Type::Map { key: Box::new(Type::String), value: Box::new(Type::Number) });
  }

  #[test]
  fn type_ann_set_conversion() {
    let ann = TypeAnn::TypeRef { name: "Set".into(), type_args: vec![TypeAnn::Number] };
    let t = Type::from(&ann);
    assert_eq!(t, Type::Set { value: Box::new(Type::Number) });
  }
}
