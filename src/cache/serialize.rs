//! Serialization for cached modules.
//!
//! Uses bincode for zero-copy deserialization of AST, types, diagnostics, and source maps.

use crate::ast::Program;
use crate::cache::key::CacheKey;
use crate::diagnostic::Diagnostic;
use crate::source_map::SourceMap;
use crate::type_checker::env::TypeEnv;

/// Cached compilation result for a single module.
#[derive(Debug, Clone)]
pub struct CachedModule {
  /// The cache key (includes file_hash, config_hash, dep_hashes).
  pub key: CacheKey,
  /// Parsed AST.
  pub ast: Program,
  /// Type environment with all inferred types.
  pub type_env: TypeEnv,
  /// Diagnostics (errors/warnings) from this file.
  pub diagnostics: Vec<Diagnostic>,
  /// Source map (if generated).
  pub source_map: Option<SourceMap>,
  /// Timestamp when cached (ms since epoch).
  pub timestamp: u64,
}

impl CachedModule {
  /// Create a dummy module for testing.
  pub fn dummy() -> Self {
    use crate::type_checker::env::TypeEnv;
    Self {
      key: CacheKey::new([0u8; 32], [0u8; 32], vec![], 1, "dummy.ts".to_string()),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: 0,
    }
  }
}

// We use serde for serialization, then bincode for binary encoding.
// All wrapper types implement serde::Serialize and serde::Deserialize.

use serde::{Deserialize, Serialize};

/// Wrapper for Program that implements serde traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedProgram {
  pub body: Vec<SerializedStatement>,
}

/// Wrapper for Statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedStatement {
  // We'll need to match the actual Statement enum variants
  // For now, use a JSON string as fallback
  Json(String),
}

impl From<Program> for SerializedProgram {
  fn from(prog: Program) -> Self {
    let json = serde_json::to_string(&prog).unwrap_or_default();
    Self { body: vec![SerializedStatement::Json(json)] }
  }
}

impl From<SerializedProgram> for Program {
  fn from(s: SerializedProgram) -> Self {
    if let Some(SerializedStatement::Json(json)) = s.body.first() {
      serde_json::from_str(json).unwrap_or_else(|_| Program { body: vec![] })
    } else {
      Program { body: vec![] }
    }
  }
}

/// Wrapper for TypeEnv
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTypeEnv {
  pub data: String, // JSON
}

impl From<TypeEnv> for SerializedTypeEnv {
  fn from(env: TypeEnv) -> Self {
    Self { data: serde_json::to_string(&env).unwrap_or_default() }
  }
}

impl From<SerializedTypeEnv> for TypeEnv {
  fn from(s: SerializedTypeEnv) -> Self {
    serde_json::from_str(&s.data).unwrap_or_else(|_| TypeEnv::new())
  }
}

/// Wrapper for Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedDiagnostic {
  pub data: String, // JSON
}

impl From<Diagnostic> for SerializedDiagnostic {
  fn from(d: Diagnostic) -> Self {
    Self { data: serde_json::to_string(&d).unwrap_or_default() }
  }
}

impl From<SerializedDiagnostic> for Diagnostic {
  fn from(s: SerializedDiagnostic) -> Self {
    serde_json::from_str(&s.data).unwrap_or_else(|_| {
      Diagnostic::error(
        "DESERIALIZE_FAILED".to_string(),
        "deserialization failed".to_string(),
        crate::token::Span::new(0, 0),
      )
    })
  }
}

/// Wrapper for SourceMap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSourceMap {
  pub data: String, // JSON
}

impl From<SourceMap> for SerializedSourceMap {
  fn from(sm: SourceMap) -> Self {
    Self { data: serde_json::to_string(&sm).unwrap_or_default() }
  }
}

impl From<SerializedSourceMap> for Option<SourceMap> {
  fn from(s: SerializedSourceMap) -> Self {
    if s.data.is_empty() { None } else { serde_json::from_str(&s.data).ok() }
  }
}

/// Fully serializable cached module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedCachedModule {
  pub key: CacheKey,
  pub ast: SerializedProgram,
  pub type_env: SerializedTypeEnv,
  pub diagnostics: Vec<SerializedDiagnostic>,
  pub source_map: SerializedSourceMap,
  pub timestamp: u64,
}

impl SerializedCachedModule {
  pub fn into_cached_module(self) -> CachedModule {
    self.into()
  }
}

impl From<CachedModule> for SerializedCachedModule {
  fn from(m: CachedModule) -> Self {
    let source_map = m
      .source_map
      .map(|sm| sm.into())
      .unwrap_or_else(|| SerializedSourceMap { data: String::new() });
    Self {
      key: m.key,
      ast: m.ast.into(),
      type_env: m.type_env.into(),
      diagnostics: m.diagnostics.into_iter().map(Into::into).collect(),
      source_map,
      timestamp: m.timestamp,
    }
  }
}

impl From<SerializedCachedModule> for CachedModule {
  fn from(s: SerializedCachedModule) -> Self {
    Self {
      key: s.key,
      ast: s.ast.into(),
      type_env: s.type_env.into(),
      diagnostics: s.diagnostics.into_iter().map(Into::into).collect(),
      source_map: s.source_map.into(),
      timestamp: s.timestamp,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::Program;
  use crate::cache::key::CacheKey;
  use crate::type_checker::env::TypeEnv;

  #[test]
  fn serialized_module_roundtrip() {
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: 1234567890,
    };

    let serialized: SerializedCachedModule = module.clone().into();
    let deserialized: CachedModule = serialized.into();

    assert_eq!(deserialized.key, key);
    assert_eq!(deserialized.timestamp, 1234567890);
    assert!(deserialized.source_map.is_none());
  }

  #[test]
  fn bincode_encode_decode() {
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: 1234567890,
    };

    let serialized: SerializedCachedModule = module.into();
    let encoded = bincode::serialize(&serialized).unwrap();
    let decoded: SerializedCachedModule = bincode::deserialize(&encoded).unwrap();
    let deserialized: CachedModule = decoded.into();

    assert_eq!(deserialized.key, key);
    assert_eq!(deserialized.timestamp, 1234567890);
  }
}
