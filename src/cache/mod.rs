//! Incremental compilation cache.
//!
//! Caches parse + type-check results keyed by file content + config + transitive deps.
//! On cache hit, skips parse + type-check; only codegen runs.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod invalidate;
pub mod key;
pub mod serialize;
pub mod storage;

use serialize::CachedModule;
use storage::CacheStorage;

/// Cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
  /// Root cache directory (defaults to OS cache dir / eletrolitic).
  pub cache_dir: PathBuf,
  /// Maximum cache size in bytes (default 500MB).
  pub max_size: u64,
  /// Enable cache (can be disabled via --no-cache).
  pub enabled: bool,
}

impl Default for CacheConfig {
  fn default() -> Self {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("eletrolitic");
    Self {
      cache_dir,
      max_size: 500 * 1024 * 1024, // 500MB
      enabled: true,
    }
  }
}

/// In-memory cache statistics.
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
  pub entries: usize,
  pub total_bytes: u64,
  pub hits: u64,
  pub misses: u64,
}

impl CacheStats {
  pub fn hit_rate(&self) -> f64 {
    let total = self.hits + self.misses;
    if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
  }
}

/// Result of a cache lookup.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CacheLookup {
  /// Cache hit - return cached module.
  Hit(Box<CachedModule>),
  /// Cache miss - need to recompile.
  Miss,
  /// Cache disabled.
  Disabled,
}

/// Main cache interface.
pub struct Cache {
  storage: CacheStorage,
  config: CacheConfig,
  stats: CacheStats,
}

impl Cache {
  /// Create a new cache with default configuration.
  pub fn new() -> Result<Self, String> {
    Self::with_config(CacheConfig::default())
  }

  /// Create a new cache with custom configuration.
  pub fn with_config(config: CacheConfig) -> Result<Self, String> {
    let storage = CacheStorage::new(&config.cache_dir)?;
    Ok(Self { storage, config, stats: CacheStats::default() })
  }

  /// Create a disabled cache (no-op).
  pub fn disabled() -> Self {
    let config = CacheConfig { enabled: false, ..Default::default() };
    let storage = CacheStorage::new(&config.cache_dir)
      .unwrap_or_else(|_| CacheStorage::new(Path::new(".")).unwrap());
    Self { storage, config, stats: CacheStats::default() }
  }

  /// Look up a module by its cache key.
  pub fn get(&mut self, key: &CacheKey) -> CacheLookup {
    if !self.config.enabled {
      return CacheLookup::Disabled;
    }
    match self.storage.read(key) {
      Ok(Some(module)) => {
        self.stats.hits += 1;
        self.stats.entries = self.storage.entry_count();
        self.stats.total_bytes = self.storage.total_size();
        CacheLookup::Hit(Box::new(module))
      }
      Ok(None) => {
        self.stats.misses += 1;
        CacheLookup::Miss
      }
      Err(e) => {
        eprintln!("\x1b[2mℹ  cache read error: {e}\x1b[0m");
        self.stats.misses += 1;
        CacheLookup::Miss
      }
    }
  }

  /// Store a module in the cache.
  pub fn put(&mut self, key: CacheKey, module: CachedModule) -> Result<(), String> {
    if !self.config.enabled {
      return Ok(());
    }
    self.storage.write(&key, &module)?;
    // Update stats and maybe evict
    self.stats.entries = self.storage.entry_count();
    self.stats.total_bytes = self.storage.total_size();
    if self.stats.total_bytes > self.config.max_size {
      self.storage.evict_lru(self.config.max_size / 2)?;
      self.stats.entries = self.storage.entry_count();
      self.stats.total_bytes = self.storage.total_size();
    }
    Ok(())
  }

  /// Invalidate cache entries for changed files and their dependents.
  pub fn invalidate(&mut self, changed_files: &[PathBuf]) -> Result<usize, String> {
    if !self.config.enabled {
      return Ok(0);
    }
    let removed = invalidate::invalidate(&mut self.storage, changed_files)?;
    self.stats.entries = self.storage.entry_count();
    self.stats.total_bytes = self.storage.total_size();
    Ok(removed)
  }

  /// Clear all cache entries.
  pub fn clear(&mut self) -> Result<(), String> {
    self.storage.clear()?;
    self.stats = CacheStats::default();
    Ok(())
  }

  /// Get current cache statistics.
  pub fn stats(&self) -> CacheStats {
    self.stats.clone()
  }

  /// Get the cache directory path.
  pub fn cache_dir(&self) -> &Path {
    &self.config.cache_dir
  }

  /// Get total number of cache entries.
  pub fn entry_count(&self) -> usize {
    self.storage.entry_count()
  }

  /// Get total cache size in bytes.
  pub fn total_size(&self) -> u64 {
    self.storage.total_size()
  }
}

impl Default for Cache {
  fn default() -> Self {
    Self::new().unwrap_or_else(|_| Self::disabled())
  }
}

/// Build a cache key for a file.
pub fn build_cache_key(
  file_path: &Path,
  file_content: &str,
  config_hash: &[u8; 32],
  dep_keys: &[[u8; 32]],
) -> CacheKey {
  key::compute(file_path, file_content, config_hash, dep_keys)
}

/// Compute a hash of the compiler configuration.
pub fn hash_config(
  options: &crate::config::CompilerOptions,
  electrolitic_cfg: &Option<crate::config::ElectroliticConfig>,
) -> [u8; 32] {
  use blake3::Hasher;
  let mut hasher = Hasher::new();
  // Serialize relevant config fields deterministically
  let config_json = serde_json::json!({
      "target": options.target,
      "strict": options.strict,
      "module": options.module,
      "moduleResolution": options.module_resolution,
      "lib": options.lib,
      "jsx": options.jsx,
      "jsxFactory": options.jsx_factory,
      "jsxFragmentFactory": options.jsx_fragment_factory,
      "experimentalDecorators": options.experimental_decorators,
      "esModuleInterop": options.es_module_interop,
      "allowSyntheticDefaultImports": options.allow_synthetic_default_imports,
      "baseUrl": options.base_url,
      "paths": options.paths,
      // Electrolitic config overrides
      "electrolitic": {
          "target": electrolitic_cfg.as_ref().and_then(|c| c.target.clone()),
          "strict": electrolitic_cfg.as_ref().and_then(|c| c.strict),
          "module": electrolitic_cfg.as_ref().and_then(|c| c.module.clone()),
          "moduleResolution": electrolitic_cfg.as_ref().and_then(|c| c.module_resolution.clone()),
          "lib": electrolitic_cfg.as_ref().and_then(|c| c.lib.clone()),
          "jsx": electrolitic_cfg.as_ref().and_then(|c| c.jsx.clone()),
          "jsxFactory": electrolitic_cfg.as_ref().and_then(|c| c.jsx_factory.clone()),
          "jsxFragmentFactory": electrolitic_cfg.as_ref().and_then(|c| c.jsx_fragment_factory.clone()),
          "experimentalDecorators": electrolitic_cfg.as_ref().and_then(|c| c.experimental_decorators),
          "esModuleInterop": electrolitic_cfg.as_ref().and_then(|c| c.es_module_interop),
          "allowSyntheticDefaultImports": electrolitic_cfg.as_ref().and_then(|c| c.allow_synthetic_default_imports),
          "baseUrl": electrolitic_cfg.as_ref().and_then(|c| c.base_url.clone()),
          "paths": electrolitic_cfg.as_ref().and_then(|c| c.paths.clone()),
      }
  });
  let config_str = serde_json::to_string(&config_json).unwrap_or_default();
  hasher.update(config_str.as_bytes());
  *hasher.finalize().as_bytes()
}

/// Get current timestamp in milliseconds.
#[allow(dead_code)]
fn current_timestamp_ms() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// Re-export public types
pub use key::CacheKey;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::Program;
  use crate::type_checker::env::TypeEnv;
  use tempfile::tempdir;

  #[test]
  fn cache_disabled_noop() {
    let mut cache = Cache::disabled();
    let key = CacheKey::new([0u8; 32], [0u8; 32], vec![], 1, "test.ts".to_string());
    let module = CachedModule::dummy();
    assert!(cache.put(key.clone(), module).is_ok());
    assert!(matches!(cache.get(&key), CacheLookup::Disabled));
  }

  #[test]
  fn cache_basic_roundtrip() {
    let dir = tempdir().unwrap();
    let config = CacheConfig { cache_dir: dir.path().to_path_buf(), ..Default::default() };
    let mut cache = Cache::with_config(config).unwrap();

    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![[3u8; 32]], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: current_timestamp_ms(),
    };

    cache.put(key.clone(), module.clone()).unwrap();
    match cache.get(&key) {
      CacheLookup::Hit(cached) => {
        assert_eq!(cached.key, key);
        assert_eq!(cached.timestamp, module.timestamp);
      }
      _ => panic!("expected cache hit"),
    }
  }

  #[test]
  fn cache_miss() {
    let dir = tempdir().unwrap();
    let config = CacheConfig { cache_dir: dir.path().to_path_buf(), ..Default::default() };
    let mut cache = Cache::with_config(config).unwrap();

    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    assert!(matches!(cache.get(&key), CacheLookup::Miss));
  }

  #[test]
  fn cache_stats() {
    let dir = tempdir().unwrap();
    let config = CacheConfig { cache_dir: dir.path().to_path_buf(), ..Default::default() };
    let mut cache = Cache::with_config(config).unwrap();

    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: current_timestamp_ms(),
    };

    assert_eq!(cache.stats().hits, 0);
    assert_eq!(cache.stats().misses, 0);

    cache.get(&key); // miss
    assert_eq!(cache.stats().misses, 1);

    cache.put(key.clone(), module).unwrap();
    cache.get(&key); // hit
    assert_eq!(cache.stats().hits, 1);
  }

  #[test]
  fn cache_clear() {
    let dir = tempdir().unwrap();
    let config = CacheConfig { cache_dir: dir.path().to_path_buf(), ..Default::default() };
    let mut cache = Cache::with_config(config).unwrap();

    let key = CacheKey::new([1u8; 32], [2u8; 32], vec![], 1, "test.ts".to_string());
    let module = CachedModule {
      key: key.clone(),
      ast: Program { body: vec![] },
      type_env: TypeEnv::new(),
      diagnostics: vec![],
      source_map: None,
      timestamp: current_timestamp_ms(),
    };

    cache.put(key.clone(), module).unwrap();
    assert_eq!(cache.stats().entries, 1);
    cache.clear().unwrap();
    assert_eq!(cache.stats().entries, 0);
    assert!(matches!(cache.get(&key), CacheLookup::Miss));
  }

  #[test]
  fn hash_config_deterministic() {
    let opts = crate::config::CompilerOptions::default();
    let cfg = None;
    let h1 = hash_config(&opts, &cfg);
    let h2 = hash_config(&opts, &cfg);
    assert_eq!(h1, h2);
  }

  #[test]
  fn hash_config_changes_with_options() {
    let mut opts1 = crate::config::CompilerOptions::default();
    opts1.target = Some("es2020".to_string());
    let opts2 = crate::config::CompilerOptions::default();

    let h1 = hash_config(&opts1, &None);
    let h2 = hash_config(&opts2, &None);
    assert_ne!(h1, h2);
  }
}
