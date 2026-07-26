use eletrolitic::cache::{Cache, CacheConfig};
use eletrolitic::compiler::Compiler;
use eletrolitic::config::{self, CacheConfig as ElectroliticCacheConfig};
use std::time::Instant;

/// Get or create an ElectroliticConfig, returning a mutable reference.
fn ensure_cfg<'a>(
  cfg: &'a mut Option<(eletrolitic::config::ElectroliticConfig, std::path::PathBuf)>,
  cwd: &std::path::Path,
) -> &'a mut eletrolitic::config::ElectroliticConfig {
  &mut cfg
    .get_or_insert_with(|| (eletrolitic::config::ElectroliticConfig::default(), cwd.to_path_buf()))
    .0
}

fn fmt_size(bytes: usize) -> String {
  if bytes >= 1024 { format!("{:.2} kB", bytes as f64 / 1024.0) } else { format!("{bytes} B") }
}

fn handle_cache_subcommand(cache_subcommand: &str, cache_dir: Option<String>, cache_enabled: bool) {
  let cwd = std::env::current_dir().unwrap_or_default();
  let mut eletrolitic_cfg = None;
  if let Some((cfg, cfg_dir)) = config::load_electrolitic_config(&cwd) {
    eletrolitic_cfg = Some((cfg, cfg_dir));
  }
  let cache_config = {
    let mut cfg = ElectroliticCacheConfig::default();
    if let Some((ref ecfg, _)) = eletrolitic_cfg
      && let Some(ref ccfg) = ecfg.cache
    {
      cfg.enabled = ccfg.enabled;
      cfg.cache_dir = ccfg.cache_dir.clone();
      cfg.max_size = ccfg.max_size;
    }
    if cache_dir.is_some() {
      cfg.cache_dir = cache_dir.clone();
    }
    cfg.enabled = Some(cache_enabled);

    let cache_dir = cfg.cache_dir.map(std::path::PathBuf::from).unwrap_or_else(|| {
      eletrolitic_cfg.as_ref().map(|(_, d)| d.clone()).unwrap_or(cwd).join(".eletrolitic-cache")
    });

    CacheConfig {
      cache_dir,
      max_size: cfg.max_size.unwrap_or(500 * 1024 * 1024),
      enabled: cfg.enabled.unwrap_or(false),
    }
  };
  let mut cache = Cache::with_config(cache_config).unwrap_or_else(|_| Cache::disabled());
  match cache_subcommand {
    "clear" => {
      cache.clear().unwrap();
      eprintln!("\x1b[32m✔\x1b[0m Cache cleared");
    }
    "stats" => {
      // Read stats directly from storage
      let entries = cache.entry_count();
      let total_bytes = cache.total_size();
      eprintln!("\x1b[2mℹ  Cache directory: {}\x1b[0m", cache.cache_dir().display());
      eprintln!("\x1b[2mℹ  Entries: {}\x1b[0m", entries);
      eprintln!("\x1b[2mℹ  Total size: {:.2} MB\x1b[0m", total_bytes as f64 / 1024.0 / 1024.0);
      // In-memory stats (hits/misses) reset per invocation; show storage stats instead
      eprintln!("\x1b[2mℹ  (Hits/Misses reset per CLI invocation)\x1b[0m");
    }
    "dir" => {
      eprintln!("{}", cache.cache_dir().display());
    }
    _ => {
      eprintln!("Unknown cache subcommand: {cache_subcommand}");
      eprintln!("Usage: eletrolitic cache [clear|stats|dir]");
      std::process::exit(1);
    }
  }
  std::process::exit(0);
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let mut entry = None;
  let mut tsconfig = None;
  let mut out_dir: Option<String> = None;
  let mut no_emit = false;
  let mut target: Option<String> = None;
  let mut strict = false;
  let mut cache_enabled = true;
  let mut cache_dir: Option<String> = None;
  let mut cache_subcommand: Option<String> = None;

  let mut i = 1;
  while i < args.len() {
    match args[i].as_str() {
      "--help" | "-h" => {
        eprintln!("Usage: eletrolitic [options] [file.ts]");
        eprintln!("       eletrolitic build|compile");
        eprintln!("       eletrolitic cache [clear|stats|dir]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --tsconfig <path>  Path to tsconfig.json");
        eprintln!("  --outDir <dir>     Output directory");
        eprintln!("  --noEmit           Type-check only, no output");
        eprintln!("  --target <target>  ES target (es2015-esnext)");
        eprintln!("  --strict           Enable strict mode");
        eprintln!("  --cache            Enable incremental cache (default)");
        eprintln!("  --no-cache         Disable incremental cache");
        eprintln!("  --cache-dir <dir>  Custom cache directory");
        eprintln!("  --help, -h         Show this help");
        eprintln!("  --version, -V      Show version");
        eprintln!();
        eprintln!("Config: eletrolitic.config.ts with defineConfig({{ entry: [...] }})");
        std::process::exit(0);
      }
      "--version" | "-V" => {
        eprintln!("eletrolitic 0.1.0");
        std::process::exit(0);
      }
      "--tsconfig" => {
        i += 1;
        tsconfig = args.get(i).cloned();
      }
      "--outDir" => {
        i += 1;
        out_dir = args.get(i).cloned();
      }
      "--noEmit" => {
        no_emit = true;
      }
      "--target" => {
        i += 1;
        target = args.get(i).cloned();
      }
      "--strict" => {
        strict = true;
      }
      "--cache" => {
        cache_enabled = true;
      }
      "--no-cache" => {
        cache_enabled = false;
      }
      "--cache-dir" => {
        i += 1;
        cache_dir = args.get(i).cloned();
      }
      "build" | "compile" => {}
      "cache" => {
        i += 1;
        cache_subcommand = args.get(i).cloned();
      }
      _ if entry.is_none() => entry = Some(args[i].clone()),
      _ => {}
    }
    i += 1;
  }

  // Handle cache subcommand first (doesn't require entry)
  if let Some(subcmd) = cache_subcommand {
    handle_cache_subcommand(&subcmd, cache_dir.clone(), cache_enabled);
  }

  // Auto-detect config: try eletrolitic.config.ts/js/json in cwd, then entry's dir
  let mut eletrolitic_cfg = None;
  let cwd = std::env::current_dir().unwrap_or_default();
  let mut cfg_dir = cwd.clone();
  if let Some((cfg, dir)) = config::load_electrolitic_config(&cwd) {
    eprintln!("\x1b[2mℹ  eletrolitic\x1b[0m");
    for name in &["eletrolitic.config.ts", "eletrolitic.config.js", "eletrolitic.config.json"] {
      if dir.join(name).exists() {
        eprintln!("\x1b[2mℹ  config file: {}\x1b[0m", dir.join(name).display());
        break;
      }
    }
    cfg_dir = dir.clone();
    eletrolitic_cfg = Some((cfg, dir));
  } else if let Some(ref entry_path) = entry {
    // Try entry's directory
    if let Some(parent) = std::path::Path::new(entry_path).parent()
      && let Some((cfg, dir)) = config::load_electrolitic_config(parent)
    {
      eprintln!("\x1b[2mℹ  eletrolitic\x1b[0m");
      for name in &["eletrolitic.config.ts", "eletrolitic.config.js", "eletrolitic.config.json"] {
        if dir.join(name).exists() {
          eprintln!("\x1b[2mℹ  config file: {}\x1b[0m", dir.join(name).display());
          break;
        }
      }
      cfg_dir = dir.clone();
      eletrolitic_cfg = Some((cfg, dir));
    }
  }

  // CLI flags override config values
  if let Some(dir) = &out_dir {
    ensure_cfg(&mut eletrolitic_cfg, &cwd).out_dir = Some(dir.clone());
  }
  if strict {
    ensure_cfg(&mut eletrolitic_cfg, &cwd).strict = Some(true);
  }
  if let Some(t) = &target {
    ensure_cfg(&mut eletrolitic_cfg, &cwd).target = Some(t.clone());
  }

  // If no entry given, get it from eletrolitic config
  if entry.is_none()
    && let Some((ref cfg, _)) = eletrolitic_cfg
    && let Some(ref entries) = cfg.entry
    && let Some(first) = entries.first()
  {
    entry = Some(first.clone());
  }

  let Some(entry) = entry else {
    eprintln!("Usage: eletrolitic <file.ts> [--tsconfig <path>]");
    eprintln!("Or create a eletrolitic.config.ts with entry points.");
    std::process::exit(1);
  };

  eprintln!("\x1b[2mℹ  entry: {entry}\x1b[0m");

  let start = Instant::now();

  // Build cache config: config file -> CLI flags
  let cache_config = {
    let mut cfg = ElectroliticCacheConfig::default();
    if let Some((ref ecfg, _)) = eletrolitic_cfg
      && let Some(ref ccfg) = ecfg.cache
    {
      cfg.enabled = ccfg.enabled;
      cfg.cache_dir = ccfg.cache_dir.clone();
      cfg.max_size = ccfg.max_size;
    }
    // CLI overrides
    if cache_dir.is_some() {
      cfg.cache_dir = cache_dir.clone();
    }
    // CLI --cache/--no-cache overrides config
    cfg.enabled = Some(cache_enabled);

    // Resolve final cache dir: config -> project root (.eletrolitic-cache) -> cwd
    let cache_dir = cfg
      .cache_dir
      .map(std::path::PathBuf::from)
      .unwrap_or_else(|| cfg_dir.join(".eletrolitic-cache"));

    CacheConfig {
      cache_dir,
      max_size: cfg.max_size.unwrap_or(500 * 1024 * 1024),
      enabled: cfg.enabled.unwrap_or(false),
    }
  };

  let result = if cache_config.enabled {
    let mut compiler = Compiler::with_cache(cache_config.clone());
    if let Some(cfg) = tsconfig {
      compiler.compile_with_tsconfig_instance(&entry, &cfg)
    } else {
      compiler.compile_instance(&entry)
    }
  } else {
    if let Some(cfg) = tsconfig {
      Compiler::compile_with_tsconfig(&entry, &cfg)
    } else {
      Compiler::compile(&entry)
    }
  };

  match result {
    Ok(outputs) => {
      if no_emit {
        let file_count = outputs.iter().filter(|(p, _)| p.ends_with(".js")).count();
        eprintln!("\x1b[2mℹ  {} files (no emit)\x1b[0m", file_count);
      } else {
        let mut total_bytes = 0usize;
        for (path, content) in &outputs {
          let write_path = if let Some(dir) = &out_dir {
            let file_name = std::path::Path::new(path)
              .file_name()
              .map(|f| f.to_string_lossy().to_string())
              .unwrap_or_else(|| path.clone());
            format!("{}/{}", dir, file_name)
          } else {
            path.clone()
          };
          if let Some(parent) = std::path::Path::new(&write_path).parent() {
            let _ = std::fs::create_dir_all(parent);
          }
          if let Err(e) = std::fs::write(&write_path, content) {
            eprintln!("Error writing {write_path}: {e}");
            std::process::exit(1);
          }
          total_bytes += content.len();
          let display = std::path::Path::new(&write_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| write_path.clone());
          eprintln!("\x1b[2mℹ  {display:<32} {}\x1b[0m", fmt_size(content.len()));
        }
        let elapsed = start.elapsed().as_millis();
        let file_count = outputs.iter().filter(|(p, _)| p.ends_with(".js")).count();
        eprintln!("\x1b[2mℹ  {} files, total: {}\x1b[0m", file_count, fmt_size(total_bytes));
        eprintln!("\x1b[32m✔\x1b[0m Build complete in {elapsed}ms");
      }
    }
    Err(errors) => {
      for err in &errors {
        eprintln!("{err}");
      }
      std::process::exit(1);
    }
  }
}
