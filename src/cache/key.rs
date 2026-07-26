//! Cache key computation.
//!
//! CacheKey uniquely identifies a compilation unit by:
//! - File content hash (blake3)
//! - Config hash (blake3 of serialized compiler options)
//! - Transitive dependency hashes
//! - Schema version (for forward compatibility)

use blake3::Hasher;
use hex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Cache key uniquely identifying a compilation result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
  /// Hash of file contents.
  pub file_hash: [u8; 32],
  /// Hash of compiler configuration.
  pub config_hash: [u8; 32],
  /// Hashes of transitive dependencies (file_hash of each dep).
  pub dep_hashes: Vec<[u8; 32]>,
  /// Schema version (bump on format change).
  pub schema_version: u8,
  /// Source file path (for invalidation).
  pub source_path: String,
}

impl CacheKey {
  /// Create a new cache key.
  pub fn new(
    file_hash: [u8; 32],
    config_hash: [u8; 32],
    dep_hashes: Vec<[u8; 32]>,
    schema_version: u8,
    source_path: String,
  ) -> Self {
    Self { file_hash, config_hash, dep_hashes, schema_version, source_path }
  }

  /// Compute a stable key hash for use as filename.
  pub fn key_hash(&self) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&self.file_hash);
    hasher.update(&self.config_hash);
    for dep in &self.dep_hashes {
      hasher.update(dep);
    }
    hasher.update(&[self.schema_version]);
    *hasher.finalize().as_bytes()
  }

  /// Get the shard directory name (first 2 hex chars).
  pub fn shard(&self) -> String {
    let key_hash = self.key_hash();
    hex::encode(&key_hash[..1])
  }

  /// Get the object filename (remaining hex chars + .bin).
  pub fn object_name(&self) -> String {
    let key_hash = self.key_hash();
    format!("{}.bin", hex::encode(&key_hash[1..]))
  }
}

/// Compute cache key for a file.
pub fn compute(
  file_path: &Path,
  file_content: &str,
  config_hash: &[u8; 32],
  dep_hashes: &[[u8; 32]],
) -> CacheKey {
  let file_hash = hash_content(file_content);
  let source_path = file_path.to_string_lossy().to_string();
  CacheKey::new(file_hash, *config_hash, dep_hashes.to_vec(), 1, source_path)
}

/// Hash file content with blake3.
pub fn hash_content(content: &str) -> [u8; 32] {
  let mut hasher = Hasher::new();
  hasher.update(content.as_bytes());
  *hasher.finalize().as_bytes()
}

/// Hash a list of dependency file hashes into a single combined hash.
pub fn combine_dep_hashes(dep_hashes: &[[u8; 32]]) -> [u8; 32] {
  let mut hasher = Hasher::new();
  for h in dep_hashes {
    hasher.update(h);
  }
  *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn key_hash_deterministic() {
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![[3u8; 32]], 1, "test.ts".to_string());
    let h1 = key.key_hash();
    let h2 = key.key_hash();
    assert_eq!(h1, h2);
  }

  #[test]
  fn key_hash_changes_with_file_hash() {
    let key1 = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "a.ts".to_string());
    let key2 = CacheKey::new([2u8; 32], [2u8; 32], vec![], 1, "b.ts".to_string());
    assert_ne!(key1.key_hash(), key2.key_hash());
  }

  #[test]
  fn key_hash_changes_with_config_hash() {
    let key1 = CacheKey::new([1u8; 32], [1u8; 32], vec![], 1, "test.ts".to_string());
    let key2 = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    assert_ne!(key1.key_hash(), key2.key_hash());
  }

  #[test]
  fn key_hash_changes_with_deps() {
    let key1 = CacheKey::new([1u8; 32], [1u8; 32], vec![], 1, "test.ts".to_string());
    let key2 = CacheKey::new([1u8; 32], [1u8; 32], vec![[2u8; 32]], 1, "test.ts".to_string());
    assert_ne!(key1.key_hash(), key2.key_hash());
  }

  #[test]
  fn key_hash_changes_with_version() {
    let key1 = CacheKey::new([1u8; 32], [1u8; 32], vec![], 1, "test.ts".to_string());
    let key2 = CacheKey::new([1u8; 32], [1u8; 32], vec![], 2, "test.ts".to_string());
    assert_ne!(key1.key_hash(), key2.key_hash());
  }

  #[test]
  fn compute_key() {
    let path = PathBuf::from("test.ts");
    let content = "let x = 1;";
    let config_hash = [0u8; 32];
    let dep_hashes: &[[u8; 32]] = &[];

    let key = compute(&path, content, &config_hash, dep_hashes);
    assert_eq!(key.schema_version, 1);
    assert_eq!(key.config_hash, config_hash);
    assert!(key.dep_hashes.is_empty());
    // file_hash should be deterministic
    let key2 = compute(&path, content, &config_hash, dep_hashes);
    assert_eq!(key.file_hash, key2.file_hash);
  }

  #[test]
  fn hash_content_deterministic() {
    let content = "let x = 1;\nlet y = 2;";
    let h1 = hash_content(content);
    let h2 = hash_content(content);
    assert_eq!(h1, h2);
  }

  #[test]
  fn hash_content_changes() {
    let h1 = hash_content("let x = 1;");
    let h2 = hash_content("let x = 2;");
    assert_ne!(h1, h2);
  }

  #[test]
  fn shard_and_object_name() {
    let key = CacheKey::new([0xab; 32], [0xcd; 32], vec![], 1, "test.ts".to_string());
    let shard = key.shard();
    let obj = key.object_name();
    assert_eq!(shard.len(), 2); // 1 byte = 2 hex chars
    assert!(obj.ends_with(".bin"));
    assert!(obj.len() > 4); // at least some hex + .bin
  }

  #[test]
  fn test_combine_dep_hashes() {
    let deps = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
    let combined = combine_dep_hashes(&deps);
    // Should be deterministic
    let combined2 = combine_dep_hashes(&deps);
    assert_eq!(combined, combined2);
    // Order matters
    let deps_rev = vec![[3u8; 32], [2u8; 32], [1u8; 32]];
    assert_ne!(combined, combine_dep_hashes(&deps_rev));
  }
}
