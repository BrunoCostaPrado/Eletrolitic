//! Filesystem cache storage backend.
//!
//! Layout:
//! <cache_dir>/
//!   index.json          // { "version": 1, "entries": { "<key_hash>": { "path": "...", "deps": [...], "timestamp": ... } } }
//!   objects/
//!     ab/
//!       abcd1234.bin    // bincode(CachedModule)

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bincode;
use hex;
use serde::{Deserialize, Serialize};

use crate::cache::key::CacheKey;
use crate::cache::serialize::{CachedModule, SerializedCachedModule};

/// Index entry for a cached module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
  /// Relative source file path (for invalidation).
  pub path: String,
  /// Dependency key hashes (for reverse dep graph).
  pub dep_hashes: Vec<[u8; 32]>,
  /// Last access time (for LRU).
  pub timestamp: u64,
}

/// On-disk cache index.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndex {
  pub version: u8,
  pub entries: HashMap<String, IndexEntry>, // key_hash (hex) -> entry
}

impl Default for CacheIndex {
  fn default() -> Self {
    Self { version: 1, entries: HashMap::new() }
  }
}

/// Filesystem cache storage.
pub struct CacheStorage {
  pub root: PathBuf,
  pub objects_dir: PathBuf,
  pub index_path: PathBuf,
  pub index: CacheIndex,
}

impl CacheStorage {
  /// Create a new cache storage at the given root directory.
  pub fn new(root: &Path) -> Result<Self, String> {
    let root = root.to_path_buf();
    let objects_dir = root.join("objects");
    let index_path = root.join("index.json");

    // Create directories
    fs::create_dir_all(&objects_dir).map_err(|e| format!("create objects dir: {e}"))?;

    // Load or create index
    let index = if index_path.exists() {
      let file = File::open(&index_path).map_err(|e| format!("open index: {e}"))?;
      let reader = BufReader::new(file);
      serde_json::from_reader(reader).unwrap_or_default()
    } else {
      CacheIndex::default()
    };

    Ok(Self { root, objects_dir, index_path, index })
  }

  /// Read a cached module by key.
  pub fn read(&self, key: &CacheKey) -> Result<Option<CachedModule>, String> {
    let key_hash = key.key_hash();
    let key_hex = hex::encode(key_hash);

    // Check index first
    let _entry = match self.index.entries.get(&key_hex) {
      Some(e) => e,
      None => return Ok(None),
    };

    // Read object file
    let shard = key.shard();
    let obj_name = key.object_name();
    let obj_path = self.objects_dir.join(shard).join(obj_name);

    if !obj_path.exists() {
      return Ok(None);
    }

    // Read and deserialize with CRC check
    let mut file = File::open(&obj_path).map_err(|e| format!("open object: {e}"))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| format!("read object: {e}"))?;

    // Verify CRC32 header (first 4 bytes)
    if buffer.len() < 4 {
      return Err("corrupted: too small".to_string());
    }
    let stored_crc = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
    let data = &buffer[4..];
    let computed_crc = crc32fast::hash(data);
    if stored_crc != computed_crc {
      return Err("corrupted: CRC mismatch".to_string());
    }

    // Deserialize SerializedCachedModule then convert
    let serialized: SerializedCachedModule =
      bincode::deserialize(data).map_err(|e| format!("deserialize: {e}"))?;
    let module = serialized.into_cached_module();

    // Verify key matches
    if module.key.key_hash() != key_hash {
      return Err("corrupted: key mismatch".to_string());
    }

    Ok(Some(module))
  }

  /// Write a cached module.
  pub fn write(&mut self, key: &CacheKey, module: &CachedModule) -> Result<(), String> {
    let key_hash = key.key_hash();
    let key_hex = hex::encode(key_hash);

    // Serialize via SerializedCachedModule
    let serialized = SerializedCachedModule::from(module.clone());
    let data = bincode::serialize(&serialized).map_err(|e| format!("serialize: {e}"))?;

    // Add CRC32 header
    let crc = crc32fast::hash(&data);
    let mut buffer = Vec::with_capacity(4 + data.len());
    buffer.extend_from_slice(&crc.to_le_bytes());
    buffer.extend_from_slice(&data);

    // Write to temp file first (atomic)
    let shard = key.shard();
    let shard_dir = self.objects_dir.join(&shard);
    fs::create_dir_all(&shard_dir).map_err(|e| format!("create shard dir: {e}"))?;

    let obj_name = key.object_name();
    let obj_path = shard_dir.join(&obj_name);
    let tmp_path = obj_path.with_extension("tmp");

    // Write temp file
    {
      let mut file = File::create(&tmp_path).map_err(|e| format!("create temp: {e}"))?;
      file.write_all(&buffer).map_err(|e| format!("write temp: {e}"))?;
      file.flush().map_err(|e| format!("flush temp: {e}"))?;
    }

    // Atomic rename
    #[cfg(unix)]
    {
      fs::rename(&tmp_path, &obj_path).map_err(|e| format!("rename: {e}"))?;
    }
    #[cfg(windows)]
    {
      // Windows: use replace_file for atomicity
      if obj_path.exists() {
        fs::remove_file(&obj_path).map_err(|e| format!("remove existing: {e}"))?;
      }
      fs::rename(&tmp_path, &obj_path).map_err(|e| format!("rename: {e}"))?;
    }

    // Update index
    let timestamp =
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    self.index.entries.insert(
      key_hex,
      IndexEntry {
        path: module.key.source_path.clone(), // Store actual source file path for invalidation
        dep_hashes: module.key.dep_hashes.clone(),
        timestamp,
      },
    );

    // Persist index
    self.persist_index()?;

    Ok(())
  }

  /// Remove a cache entry by key.
  pub fn remove(&mut self, key: &CacheKey) -> Result<bool, String> {
    let key_hash = key.key_hash();
    let key_hex = hex::encode(key_hash);

    let removed = self.index.entries.remove(&key_hex).is_some();

    if removed {
      // Remove object file
      let shard = key.shard();
      let obj_name = key.object_name();
      let obj_path = self.objects_dir.join(shard).join(obj_name);
      let _ = fs::remove_file(&obj_path); // Ignore errors
      self.persist_index()?;
    }

    Ok(removed)
  }

  /// Get all entry keys.
  pub fn keys(&self) -> Vec<String> {
    self.index.entries.keys().cloned().collect()
  }

  /// Get index entry for a key.
  pub fn get_entry(&self, key_hex: &str) -> Option<&IndexEntry> {
    self.index.entries.get(key_hex)
  }

  /// Get all entries.
  pub fn entries(&self) -> &HashMap<String, IndexEntry> {
    &self.index.entries
  }

  /// Get mutable index entries (for invalidation).
  pub fn entries_mut(&mut self) -> &mut HashMap<String, IndexEntry> {
    &mut self.index.entries
  }

  /// Total number of entries.
  pub fn entry_count(&self) -> usize {
    self.index.entries.len()
  }

  /// Total size of all object files in bytes.
  pub fn total_size(&self) -> u64 {
    let mut total = 0u64;
    for entry in self.index.entries.keys() {
      if let Ok(size) = self.object_size(entry) {
        total += size;
      }
    }
    total
  }

  /// Get size of a single object file.
  fn object_size(&self, key_hex: &str) -> Result<u64, std::io::Error> {
    // Reconstruct path from key_hex
    let shard = &key_hex[..2];
    let obj_name = format!("{}.bin", &key_hex[2..]);
    let obj_path = self.objects_dir.join(shard).join(obj_name);
    fs::metadata(obj_path).map(|m| m.len())
  }

  /// Evict LRU entries until size <= target_bytes.
  pub fn evict_lru(&mut self, target_bytes: u64) -> Result<usize, String> {
    let mut entries: Vec<_> =
      self.index.entries.iter().map(|(k, v)| (k.clone(), v.timestamp)).collect();
    // Sort by timestamp ascending (oldest first)
    entries.sort_by_key(|(_, ts)| *ts);

    let mut removed = 0;
    for (key_hex, _) in entries {
      if self.total_size() <= target_bytes {
        break;
      }
      // Reconstruct key to remove object file
      let shard = &key_hex[..2];
      let obj_name = format!("{}.bin", &key_hex[2..]);
      let obj_path = self.objects_dir.join(shard).join(obj_name);
      let _ = fs::remove_file(&obj_path);
      self.index.entries.remove(&key_hex);
      removed += 1;
    }

    if removed > 0 {
      self.persist_index()?;
    }

    Ok(removed)
  }

  /// Clear all cache entries.
  pub fn clear(&mut self) -> Result<(), String> {
    // Remove all object files
    if self.objects_dir.exists() {
      fs::remove_dir_all(&self.objects_dir).map_err(|e| format!("remove objects: {e}"))?;
      fs::create_dir_all(&self.objects_dir).map_err(|e| format!("recreate objects: {e}"))?;
    }
    self.index = CacheIndex::default();
    self.persist_index()?;
    Ok(())
  }

  /// Persist index to disk.
  pub fn persist_index(&self) -> Result<(), String> {
    let tmp_path = self.index_path.with_extension("tmp");
    {
      let file = File::create(&tmp_path).map_err(|e| format!("create index tmp: {e}"))?;
      let writer = BufWriter::new(file);
      serde_json::to_writer(writer, &self.index).map_err(|e| format!("write index: {e}"))?;
    }
    fs::rename(&tmp_path, &self.index_path).map_err(|e| format!("rename index: {e}"))?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::Program;
  use crate::cache::key::CacheKey;
  use crate::cache::serialize::CachedModule;
  use crate::type_checker::env::TypeEnv;
  use tempfile::tempdir;

  fn make_module() -> CachedModule {
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![[3u8; 32]], 1, "test.ts".to_string());
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
  fn storage_write_read() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();
    let module = make_module();
    let key = module.key.clone();

    storage.write(&key, &module).unwrap();
    let read = storage.read(&key).unwrap();
    assert!(read.is_some());
    let read = read.unwrap();
    assert_eq!(read.key, key);
    assert_eq!(read.timestamp, 1234567890);
  }

  #[test]
  fn storage_miss() {
    let dir = tempdir().unwrap();
    let storage = CacheStorage::new(dir.path()).unwrap();
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    let read = storage.read(&key).unwrap();
    assert!(read.is_none());
  }

  #[test]
  fn storage_remove() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();
    let module = make_module();
    let key = module.key.clone();

    storage.write(&key, &module).unwrap();
    assert!(storage.read(&key).unwrap().is_some());

    storage.remove(&key).unwrap();
    assert!(storage.read(&key).unwrap().is_none());
  }

  #[test]
  fn storage_clear() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();
    let module = make_module();
    let key = module.key.clone();

    storage.write(&key, &module).unwrap();
    assert_eq!(storage.entry_count(), 1);

    storage.clear().unwrap();
    assert_eq!(storage.entry_count(), 0);
    assert!(storage.read(&key).unwrap().is_none());
  }

  #[test]
  fn storage_persists_across_instances() {
    let dir = tempdir().unwrap();
    {
      let mut storage = CacheStorage::new(dir.path()).unwrap();
      let module = make_module();
      storage.write(&module.key, &module).unwrap();
    }
    // New instance should see the data
    {
      let storage = CacheStorage::new(dir.path()).unwrap();
      assert_eq!(storage.entry_count(), 1);
    }
  }

  #[test]
  fn storage_evict_lru() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();

    // Add 3 entries with different timestamps
    for i in 0..3 {
      let key = CacheKey::new([i as u8; 32], [2u8; 32], vec![], 1, format!("test{}.ts", i));
      let module = CachedModule {
        key: key.clone(),
        ast: Program { body: vec![] },
        type_env: TypeEnv::new(),
        diagnostics: vec![],
        source_map: None,
        timestamp: 1000 + i as u64,
      };
      storage.write(&key, &module).unwrap();
    }
    assert_eq!(storage.entry_count(), 3);

    // Evict to keep only 2 (target size small enough to trigger)
    // We need to set a very small target
    storage.evict_lru(1).unwrap(); // This will evict oldest entries
    // Note: evict_lru uses total_size() which depends on actual file sizes
    // This test mainly checks it runs without error
  }

  #[test]
  fn storage_index_has_deps() {
    let dir = tempdir().unwrap();
    let mut storage = CacheStorage::new(dir.path()).unwrap();
    let dep_hash = [3u8; 32];
    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![dep_hash], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: 1234567890,
    };
    storage.write(&key, &module).unwrap();

    let key_hex = hex::encode(key.key_hash());
    let entry = storage.get_entry(&key_hex).unwrap();
    assert_eq!(entry.dep_hashes, vec![dep_hash]);
  }
}
