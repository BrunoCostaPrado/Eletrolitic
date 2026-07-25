use electrolitic::compiler::Compiler;
use electrolitic::config;
use std::time::Instant;

/// Get or create an ElectroliticConfig, returning a mutable reference.
fn ensure_cfg<'a>(cfg: &'a mut Option<(electrolitic::config::ElectroliticConfig, std::path::PathBuf)>, cwd: &std::path::Path) -> &'a mut electrolitic::config::ElectroliticConfig {
  &mut cfg.get_or_insert_with(|| (electrolitic::config::ElectroliticConfig::default(), cwd.to_path_buf())).0
}

fn fmt_size(bytes: usize) -> String {
  if bytes >= 1024 { format!("{:.2} kB", bytes as f64 / 1024.0) } else { format!("{bytes} B") }
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let mut entry = None;
  let mut tsconfig = None;
  let mut out_dir: Option<String> = None;
  let mut no_emit = false;
  let mut target: Option<String> = None;
  let mut strict = false;

  let mut i = 1;
  while i < args.len() {
    match args[i].as_str() {
      "--help" | "-h" => {
        eprintln!("Usage: electrolitic [options] [file.ts]");
        eprintln!("       electrolitic build|compile");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --tsconfig <path>  Path to tsconfig.json");
        eprintln!("  --outDir <dir>     Output directory");
        eprintln!("  --noEmit           Type-check only, no output");
        eprintln!("  --target <target>  ES target (es2015-esnext)");
        eprintln!("  --strict           Enable strict mode");
        eprintln!("  --help, -h         Show this help");
        eprintln!("  --version, -V      Show version");
        eprintln!();
        eprintln!("Config: electrolitic.config.ts with defineConfig({{ entry: [...] }})");
        std::process::exit(0);
      }
      "--version" | "-V" => {
        eprintln!("electrolitic 0.1.0");
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
      "build" | "compile" => {}
      _ if entry.is_none() => entry = Some(args[i].clone()),
      _ => {}
    }
    i += 1;
  }

  // Auto-detect config: try electrolitic.config.ts/js/json in cwd, then entry's dir
  let mut electrolitic_cfg = None;
  let cwd = std::env::current_dir().unwrap_or_default();
  if let Some((cfg, cfg_dir)) = config::load_electrolitic_config(&cwd) {
    eprintln!("\x1b[2mℹ  electrolitic\x1b[0m");
    for name in &["electrolitic.config.ts", "electrolitic.config.js", "electrolitic.config.json"] {
      if cfg_dir.join(name).exists() {
        eprintln!("\x1b[2mℹ  config file: {}\x1b[0m", cfg_dir.join(name).display());
        break;
      }
    }
    electrolitic_cfg = Some((cfg, cfg_dir));
  }

  // CLI flags override config values
  if let Some(dir) = &out_dir {
    ensure_cfg(&mut electrolitic_cfg, &cwd).out_dir = Some(dir.clone());
  }
  if strict {
    ensure_cfg(&mut electrolitic_cfg, &cwd).strict = Some(true);
  }
  if let Some(t) = &target {
    ensure_cfg(&mut electrolitic_cfg, &cwd).target = Some(t.clone());
  }

  // If no entry given, get it from electrolitic config
  if entry.is_none() {
    if let Some((ref cfg, _)) = electrolitic_cfg {
      if let Some(ref entries) = cfg.entry {
        if let Some(first) = entries.first() {
          entry = Some(first.clone());
        }
      }
    }
  }

  let Some(entry) = entry else {
    eprintln!("Usage: electrolitic <file.ts> [--tsconfig <path>]");
    eprintln!("Or create a electrolitic.config.ts with entry points.");
    std::process::exit(1);
  };

  eprintln!("\x1b[2mℹ  entry: {entry}\x1b[0m");

  let start = Instant::now();

  let result = if let Some(cfg) = tsconfig {
    Compiler::compile_with_tsconfig(&entry, &cfg)
  } else {
    Compiler::compile(&entry)
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
