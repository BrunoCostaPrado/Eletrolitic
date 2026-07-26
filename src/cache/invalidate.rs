//! Cache invalidation via reverse dependency graph.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::cache::storage::CacheStorage;

/// Invalidate cache entries for changed files and their dependents.
pub fn invalidate(
  storage: &mut CacheStorage,
  changed_files: &[PathBuf], // absolute paths
) -> Result<usize, String> {
  // Build reverse dependency graph from index
  // key_hash -> set of dependent key_hashes
  let mut reverse_deps: HashMap<String, HashSet<String>> = HashMap::new();
  let mut file_to_key: HashMap<String, String> = HashMap::new(); // file path -> key_hash

  for (key_hex, entry) in storage.entries() {
    // Map file path to key
    file_to_key.insert(entry.path.clone(), key_hex.clone());

    // Add reverse edges: each dep -> this key
    for dep_hash in &entry.dep_hashes {
      let dep_hex = hex::encode(dep_hash);
      reverse_deps.entry(dep_hex).or_default().insert(key_hex.clone());
    }
  }

  // Find initial keys to invalidate (changed files)
  let mut to_invalidate = VecDeque::new();
  for file in changed_files {
    let file_str = file.to_string_lossy().to_string();
    if let Some(key_hex) = file_to_key.get(&file_str) {
      to_invalidate.push_back(key_hex.clone());
    }
  }

  // BFS through reverse deps
  let mut invalidated = HashSet::new();
  while let Some(key_hex) = to_invalidate.pop_front() {
    if invalidated.insert(key_hex.clone()) {
      // Add dependents
      if let Some(deps) = reverse_deps.get(&key_hex) {
        for dep in deps {
          to_invalidate.push_back(dep.clone());
        }
      }
    }
  }

  // Remove from storage
  let mut count = 0;
  for key_hex in invalidated {
    // Reconstruct CacheKey to remove object file
    // We only have key_hex, need to parse it back
    // For removal, we just need to delete the object file
    let shard = &key_hex[..2];
    let obj_name = format!("{}.bin", &key_hex[2..]);
    let obj_path = storage.objects_dir.join(shard).join(obj_name);
    let _ = std::fs::remove_file(&obj_path);
    storage.entries_mut().remove(&key_hex);
    count += 1;
  }

  if count > 0 {
    storage.persist_index()?;
  }

  Ok(count)
}

/// Get all keys that depend on a given file (transitive).
pub fn dependents_of(storage: &CacheStorage, file_path: &str) -> Vec<String> {
  let mut reverse_deps: HashMap<String, HashSet<String>> = HashMap::new();
  let mut file_to_key: HashMap<String, String> = HashMap::new();

  for (key_hex, entry) in storage.entries() {
    file_to_key.insert(entry.path.clone(), key_hex.clone());
    for dep_hash in &entry.dep_hashes {
      let dep_hex = hex::encode(dep_hash);
      reverse_deps.entry(dep_hex).or_default().insert(key_hex.clone());
    }
  }

  let mut result = Vec::new();
  let mut queue = VecDeque::new();
  if let Some(key_hex) = file_to_key.get(file_path) {
    queue.push_back(key_hex.clone());
  }

  let mut visited = HashSet::new();
  while let Some(key_hex) = queue.pop_front() {
    if visited.insert(key_hex.clone()) {
      result.push(key_hex.clone());
      if let Some(deps) = reverse_deps.get(&key_hex) {
        for dep in deps {
          queue.push_back(dep.clone());
        }
      }
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use crate::ast::Program;
  use crate::cache::key::CacheKey;
  use crate::cache::serialize::CachedModule;
  use crate::cache::storage::CacheStorage;
  use crate::type_checker::env::TypeEnv;
  use tempfile::tempdir;

  fn make_module(key: CacheKey) -> CachedModule {
    CachedModule {
      key,
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: 1234567890,
    }
  }

  #[test]
  fn invalidation_single_file() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();

    // Create a module for file A
    let key_a = CacheKey::new([1u8; 32], [0u8; 32], vec![], 1, "/absolute/path/a.ts".to_string());
    storage.write(&key_a, &make_module(key_a.clone())).unwrap();

    // Invalidate A
    let changed = vec![std::path::PathBuf::from("/absolute/path/a.ts")];
    let affected = crate::cache::invalidate::invalidate(&mut storage, &changed).unwrap();
    assert_eq!(affected, 1);
    assert!(storage.read(&key_a).unwrap().is_none());
  }

  #[test]
  fn reverse_deps_builds_correctly() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();

    // dep (types.ts) -> key hash = [1,0,0...]
    let dep_key =
      CacheKey::new([1u8; 32], [0u8; 32], vec![], 1, "/absolute/path/types.ts".to_string());
    // main imports dep -> dep_hashes = [[1,0,0...]]
    let main_key =
      CacheKey::new([2u8; 32], [0u8; 32], vec![[1u8; 32]], 1, "/absolute/path/main.ts".to_string());

    storage.write(&dep_key, &make_module(dep_key.clone())).unwrap();
    storage.write(&main_key, &make_module(main_key.clone())).unwrap();

    // Check index has dep_hashes
    let dep_hex = hex::encode(dep_key.key_hash());
    let main_hex = hex::encode(main_key.key_hash());

    let dep_entry = storage.get_entry(&dep_hex).unwrap();
    assert!(dep_entry.dep_hashes.is_empty());

    let main_entry = storage.get_entry(&main_hex).unwrap();
    assert_eq!(main_entry.dep_hashes.len(), 1);
    assert_eq!(main_entry.dep_hashes[0], [1u8; 32]);
  }
}
